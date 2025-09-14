// GPhoto2-based camera control for Canon EOS DSLRs on Raspberry Pi.

use actix_files as fs;
use actix_web::{web, App, HttpServer};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tokio::signal;
use tokio::sync::mpsc;

// Module imports
mod config;
mod errors;
mod gphoto_camera;
mod printers;
mod routes;
mod session;
mod templates;

use config::Config;
use printers::new_printer;
use routes::{
    camera_page, capture_image, companion_page, copies_page, create_session, generate_story,
    get_session, land_page, name_entry_page, photo_page, preview_print, preview_stream,
    print_photo, save_session_final, start_page, update_session, weapon_page,
};
use tracing::{error, info, warn};

fn spawn_gphoto_camera(
    config: config::CameraConfig,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
    gphoto_camera: Arc<Mutex<Option<Arc<gphoto_camera::GPhotoCamera>>>>,
) -> tokio::task::JoinHandle<()> {
    info!("Using GPhoto2 camera interface for DSLR");
    tokio::spawn(async move {
        // Override device to use v4l2loopback device for GPhoto preview
        let mut gphoto_config = config.clone();
        gphoto_config.v4l2_loopback_device =
            std::env::var("V4L2_LOOPBACK_DEVICE").unwrap_or_else(|_| "/dev/video2".to_string());
        info!(
            "GPhoto2 will stream preview to: {}",
            gphoto_config.v4l2_loopback_device
        );

        // Create and initialize the GPhoto camera
        let camera_arc = match gphoto_camera::GPhotoCamera::new(gphoto_config) {
            Ok(camera) => {
                // Initialize the camera
                match camera.initialize().await {
                    Ok(_) => {
                        info!("GPhoto2 camera initialized successfully");
                        let arc = Arc::new(camera);
                        // Store the camera in the shared mutex
                        *gphoto_camera.lock().unwrap() = Some(arc.clone());
                        arc
                    }
                    Err(e) => {
                        error!("Failed to initialize GPhoto2 camera: {}", e);
                        return;
                    }
                }
            }
            Err(e) => {
                error!("Failed to create GPhoto2 camera: {}", e);
                return;
            }
        };

        let (tx, mut rx) = mpsc::channel(10);

        // Start camera stream
        let camera_stream = camera_arc.clone();
        let camera_handle = tokio::spawn(async move {
            info!("Starting GPhoto2 camera preview stream");
            if let Err(e) = camera_stream
                .start_preview_stream(tx, last_frame.clone())
                .await
            {
                error!("GPhoto2 camera stream error: {}", e);
            }
        });

        // Drain the channel to prevent backpressure
        let mut frame_count = 0;
        while let Some(_frame) = rx.recv().await {
            frame_count += 1;
            if frame_count % 100 == 0 {
                info!("Received {} frames from GPhoto2 camera", frame_count);
            }
            // Frames are already stored in last_frame by the camera
        }

        warn!("GPhoto2 camera frame receiver loop ended");
        let _ = camera_handle.await;
    })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Configuration error: {}", e),
            ));
        }
    };

    info!("Starting photo booth server on {}", config.socket_addr());

    // Ensure database directory exists
    if let Some(parent) = config.database.path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    // Connect to database
    let connection_pool = SqlitePool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to database");

    // Run database migrations
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to run migrations");

    info!("Database connected and migrations completed");

    let last_frame = Arc::new(Mutex::new(None::<Vec<u8>>));

    let printer = match new_printer().await {
        Ok(p) => Some(p),
        Err(e) => {
            warn!("Failed to initialize printer: {}", e);
            None
        }
    };

    let gphoto_camera: Arc<Mutex<Option<Arc<gphoto_camera::GPhotoCamera>>>> =
        Arc::new(Mutex::new(None));

    let camera = {
        info!(
            "Initializing GPhoto2 camera with config: {:?}",
            config.camera
        );
        Some(spawn_gphoto_camera(
            config.camera.clone(),
            last_frame.clone(),
            gphoto_camera.clone(),
        ))
    };

    let server_config = config.clone();
    let gphoto_for_shutdown = gphoto_camera.clone();

    let server = HttpServer::new(move || {
        let app_config = server_config.clone();
        let db_pool = connection_pool.clone();
        let mut app = App::new()
            .app_data(web::Data::new(last_frame.clone()))
            .app_data(web::Data::new(app_config.clone()))
            .app_data(web::Data::new(db_pool))
            .app_data(web::Data::new(gphoto_camera.clone()));

        app = app
            .service(start_page)
            .service(name_entry_page)
            .service(copies_page)
            .service(camera_page)
            .service(preview_stream)
            .service(capture_image)
            .service(photo_page)
            .service(create_session)
            .service(get_session)
            .service(update_session)
            .service(generate_story)
            .service(save_session_final)
            .service(weapon_page)
            .service(land_page)
            .service(companion_page)
            .service(fs::Files::new("/images", app_config.images_path()).show_files_listing())
            .service(
                fs::Files::new("/static", app_config.storage.static_path.clone())
                    .show_files_listing(),
            );

        if let Some(p) = printer.clone() {
            app = app
                .app_data(web::Data::new(p))
                .service(print_photo)
                .service(preview_print);
        }

        app
    })
    .bind(config.socket_addr())?
    .shutdown_timeout(5)
    .run();

    // Run server with graceful shutdown
    let server_handle = server.handle();

    // Spawn the server
    let server_task = tokio::spawn(async move { server.await });

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Initiating graceful shutdown...");

    // Stop the server gracefully
    server_handle.stop(true).await;

    // Clean up the camera
    if let Some(camera_arc) = gphoto_for_shutdown.lock().unwrap().take() {
        info!("Cleaning up GPhoto camera...");
        // Dropping the Arc will trigger the Drop impl
        drop(camera_arc);
        // Give it a moment to clean up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Kill any remaining gphoto processes (belt and suspenders approach)
    let _ = std::process::Command::new("pkill")
        .args(&["-f", "gphoto2"])
        .output();
    let _ = std::process::Command::new("pkill")
        .args(&["-f", "ffmpeg.*v4l2"])
        .output();

    info!("Graceful shutdown complete");

    // Wait for server to finish
    server_task.await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Server task error: {}", e),
        )
    })??;

    Ok(())
}
