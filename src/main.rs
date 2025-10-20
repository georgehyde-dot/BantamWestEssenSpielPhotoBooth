// GPhoto2-based camera control for Canon EOS DSLRs on Raspberry Pi.

use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

// Module imports
mod config;
mod errors;
mod gphoto_camera;
mod printers;
mod routes;
mod session;
mod templates;

use config::Config;
use errors::AppError;

// ============================================================================
// Application State
// ============================================================================

/// Centralized application state container
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db_pool: SqlitePool,
    pub camera: Arc<Mutex<Option<Arc<gphoto_camera::GPhotoCamera>>>>,
    pub printer: Option<Arc<dyn printers::Printer + Send + Sync>>,
}

impl AppState {
    /// Create a new application state instance
    async fn new(config: Config) -> Result<Self, AppError> {
        info!("Initializing application state");

        // Initialize database
        let db_pool = Self::initialize_database(&config.database).await?;

        // Initialize printer (non-critical)
        let printer = Self::initialize_printer().await;

        // Camera will be initialized separately due to its async nature
        let camera = Arc::new(Mutex::new(None));

        Ok(Self {
            config,
            db_pool,
            camera,
            printer,
        })
    }

    async fn initialize_database(
        db_config: &config::DatabaseConfig,
    ) -> Result<SqlitePool, AppError> {
        info!("Initializing database at: {:?}", db_config.path);

        // Ensure database directory exists
        if let Some(parent) = db_config.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Initialization(format!("Failed to create database directory: {}", e))
            })?;
        }

        // Connect to database
        let pool = SqlitePool::connect(&db_config.connection_string())
            .await
            .map_err(|e| {
                AppError::Initialization(format!("Failed to connect to database: {}", e))
            })?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| AppError::Initialization(format!("Failed to run migrations: {}", e)))?;

        info!("Database connected and migrations completed");
        Ok(pool)
    }

    async fn initialize_printer() -> Option<Arc<dyn printers::Printer + Send + Sync>> {
        match printers::new_printer().await {
            Ok(printer) => {
                info!("Printer initialized successfully");
                Some(printer)
            }
            Err(e) => {
                warn!("Printer initialization failed (non-critical): {}", e);
                warn!("Photo booth will operate without printing capability");
                None
            }
        }
    }
}

// ============================================================================
// Camera Initialization
// ============================================================================

async fn initialize_camera(
    config: config::CameraConfig,
    camera_ref: Arc<Mutex<Option<Arc<gphoto_camera::GPhotoCamera>>>>,
) -> Result<(), AppError> {
    info!("Initializing GPhoto2 camera with config: {:?}", config);

    // Override device to use v4l2loopback device if specified
    let mut camera_config = config.clone();
    if let Ok(device) = std::env::var("V4L2_LOOPBACK_DEVICE") {
        info!("Overriding v4l2 device from environment: {}", device);
        camera_config.v4l2_loopback_device = device;
    }

    info!(
        "GPhoto2 will stream preview to: {}",
        camera_config.v4l2_loopback_device
    );

    // Create and initialize camera
    let camera = gphoto_camera::GPhotoCamera::new(camera_config)
        .map_err(|e| AppError::Initialization(format!("Failed to create GPhoto2 camera: {}", e)))?;

    camera.initialize().await.map_err(|e| {
        AppError::Initialization(format!("Failed to initialize GPhoto2 camera: {}", e))
    })?;

    info!("GPhoto2 camera initialized successfully");

    let camera_arc = Arc::new(camera);

    // Store camera reference
    {
        let mut guard = camera_ref.lock().unwrap();
        *guard = Some(camera_arc.clone());
    }

    // Start preview stream in background
    let camera_for_stream = camera_arc.clone();
    tokio::spawn(async move {
        info!("Starting GPhoto2 camera preview stream");
        if let Err(e) = camera_for_stream.start_preview_stream().await {
            error!("GPhoto2 camera stream error: {}", e);
        }
    });

    Ok(())
}

// ============================================================================
// Shutdown Handling
// ============================================================================

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

async fn cleanup_resources(state: AppState) {
    info!("Beginning resource cleanup");

    // Clean up camera
    if let Some(camera) = state.camera.lock().unwrap().take() {
        info!("Cleaning up GPhoto camera...");
        // Dropping the Arc will trigger the Drop implementation
        drop(camera);
        // Allow time for cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Close database connections
    state.db_pool.close().await;
    info!("Database connections closed");

    // Note: We're not using pkill anymore - resources should clean up properly via Drop
    // If we still have issues, we should fix the root cause rather than using pkill

    info!("Resource cleanup complete");
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ========================================
    // Phase 1: Basic Initialization
    // ========================================

    // Initialize tracing/logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting photo booth application");

    // ========================================
    // Phase 2: Configuration & State Setup
    // ========================================

    // Load configuration
    let config = Config::from_env().map_err(|e| {
        error!("Configuration error: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to load configuration: {}", e),
        )
    })?;

    info!("Configuration loaded successfully");
    info!("Server will bind to: {}", config.socket_addr());

    // Initialize application state
    let app_state = AppState::new(config.clone()).await.map_err(|e| {
        error!("Application initialization error: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to initialize application: {}", e),
        )
    })?;

    // ========================================
    // Phase 3: Camera Initialization
    // ========================================

    // Initialize camera (critical component)
    initialize_camera(config.camera.clone(), app_state.camera.clone())
        .await
        .map_err(|e| {
            error!("Camera initialization failed: {}", e);
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Camera is required for photo booth operation: {}", e),
            )
        })?;

    // ========================================
    // Phase 4: HTTP Server Setup
    // ========================================

    let socket_addr = config.socket_addr();
    let app_state_for_server = app_state.clone();

    let server = HttpServer::new(move || {
        let state = app_state_for_server.clone();
        let mut app = App::new()
            // Middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            // Application state
            .app_data(web::Data::new(state.config.clone()))
            .app_data(web::Data::new(state.db_pool.clone()))
            .app_data(web::Data::new(state.camera.clone()));

        // Core routes
        app = app
            // Session management
            .service(routes::create_session)
            .service(routes::get_session)
            .service(routes::update_session)
            .service(routes::save_session_final)
            // Page routes
            .service(routes::start_page)
            .service(routes::name_entry_page)
            .service(routes::email_entry_page)
            .service(routes::class_page)
            .service(routes::choice_page)
            .service(routes::copies_page)
            .service(routes::camera_page)
            .service(routes::photo_page)
            .service(routes::thank_you_page)
            // Camera functionality
            .service(routes::preview_stream)
            .service(routes::capture_image)
            .service(routes::test_stream)
            // Story generation
            .service(routes::generate_story)
            // Static file serving
            .service(fs::Files::new("/images", state.config.images_path()).show_files_listing())
            .service(
                fs::Files::new("/static", state.config.storage.static_path.clone())
                    .show_files_listing(),
            );

        // Conditional printer routes
        if let Some(printer) = state.printer {
            app = app
                .app_data(web::Data::new(printer))
                .service(routes::print_photo)
                .service(routes::preview_print);
        }

        app
    })
    .bind(socket_addr)?
    .shutdown_timeout(5)
    .run();

    let server_handle = server.handle();
    let server_task = tokio::spawn(async move { server.await });

    info!("Photo booth server started on {}", socket_addr);
    info!("System ready for operation");

    // ========================================
    // Phase 5: Run Until Shutdown
    // ========================================

    shutdown_signal().await;

    // ========================================
    // Phase 6: Graceful Shutdown
    // ========================================

    info!("Initiating graceful shutdown...");

    // Stop accepting new connections and wait for existing ones to complete
    server_handle.stop(true).await;

    // Clean up resources
    cleanup_resources(app_state).await;

    // Wait for server task to complete
    server_task.await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Server task error: {}", e),
        )
    })??;

    info!("Graceful shutdown complete");
    Ok(())
}
