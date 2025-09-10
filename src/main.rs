// Linux (Raspberry Pi) implementation: V4L2 (v4l crate) MJPEG preview and capture over HTTP.
// Non-Linux: build a stub that informs the user this is Linux-only.

#[cfg(target_os = "linux")]
use actix_files as fs;
#[cfg(target_os = "linux")]
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
#[cfg(target_os = "linux")]
use async_stream;
#[cfg(target_os = "linux")]
use chrono;
#[cfg(target_os = "linux")]
use config::{Config, ConfigError};
#[cfg(target_os = "linux")]
use serde_json;
#[cfg(target_os = "linux")]
use sqlx::SqlitePool;
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use tokio::sync::mpsc;
#[cfg(target_os = "linux")]
use v4l::buffer::Type;
#[cfg(target_os = "linux")]
use v4l::device::Device;
#[cfg(target_os = "linux")]
use v4l::io::traits::CaptureStream;
#[cfg(target_os = "linux")]
use v4l::prelude::*;
#[cfg(target_os = "linux")]
use v4l::Format;
#[cfg(target_os = "linux")]
use v4l::FourCC;
// bring Capture trait into scope so Device.format(), Device.set_format() are available
#[cfg(target_os = "linux")]
use v4l::io::userptr;
#[cfg(target_os = "linux")]
use v4l::video::Capture;

#[cfg(target_os = "linux")]
use bytes::Bytes; // for streaming
#[cfg(target_os = "linux")]
use image;

// Module imports
mod camera;
mod config;
mod errors;
mod gphoto_camera;
mod image_processing;
mod printers;
mod routes;
mod session;
mod templates;

#[cfg(target_os = "linux")]
use printers::{new_printer, PaperSize, PrintJob, PrintQuality, Printer, PrinterError};
#[cfg(target_os = "linux")]
use routes::{
    camera_page, capture_image, companion_page, copies_page, create_session, generate_story,
    get_session, land_page, name_entry_page, photo_page, preview_print, preview_stream,
    print_photo, save_session_final, start_page, update_session, weapon_page,
};
#[cfg(target_os = "linux")]
use session::Session;
#[cfg(target_os = "linux")]
use tracing::{error, info, warn};

#[cfg(target_os = "linux")]
fn spawn_camera(
    config: config::CameraConfig,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
    gphoto_camera: Arc<Mutex<Option<Arc<gphoto_camera::GPhotoCamera>>>>,
) -> tokio::task::JoinHandle<()> {
    // Check if we should use gphoto2 (for USB-connected DSLR cameras like Canon EOS)
    let use_gphoto = std::env::var("USE_GPHOTO")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if use_gphoto {
        info!("Using GPhoto2 camera interface for DSLR");
        tokio::spawn(async move {
            // Override device to use v4l2loopback device for GPhoto preview
            let mut gphoto_config = config.clone();
            gphoto_config.device =
                std::env::var("V4L2_LOOPBACK_DEVICE").unwrap_or_else(|_| "/dev/video2".to_string());
            info!("GPhoto2 will stream preview to: {}", gphoto_config.device);

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
    } else {
        info!("Using V4L2 camera interface for device: {}", config.device);
        tokio::spawn(async move {
            let camera = camera::Camera::new(config.clone());
            let (tx, mut rx) = mpsc::channel(10);

            // Start camera stream
            let camera_handle = tokio::spawn(async move {
                info!("Starting V4L2 camera preview stream");
                if let Err(e) = camera.start_preview_stream(tx, last_frame.clone()).await {
                    error!("V4L2 camera stream error: {}", e);
                }
            });

            // Drain the channel to prevent backpressure
            let mut frame_count = 0;
            while let Some(_frame) = rx.recv().await {
                frame_count += 1;
                if frame_count % 1000 == 0 {
                    info!("Received {} frames from V4L2 camera", frame_count);
                }
                // Frames are already stored in last_frame by the camera
            }

            warn!("V4L2 camera frame receiver loop ended");
            let _ = camera_handle.await;
        })
    }
}

#[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    let printer = match new_printer().await {
        Ok(p) => Some(p),
        Err(e) => {
            warn!("Failed to initialize printer: {}", e);
            None
        }
    };

    #[cfg(target_os = "linux")]
    let gphoto_camera: Arc<Mutex<Option<Arc<gphoto_camera::GPhotoCamera>>>> =
        Arc::new(Mutex::new(None));

    #[cfg(target_os = "linux")]
    let camera = {
        let use_gphoto = std::env::var("USE_GPHOTO")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        info!(
            "Initializing camera with config: {:?} (using {})",
            config.camera,
            if use_gphoto { "GPhoto2" } else { "V4L2" }
        );
        Some(spawn_camera(
            config.camera.clone(),
            last_frame.clone(),
            gphoto_camera.clone(),
        ))
    };

    #[cfg(not(target_os = "linux"))]
    let camera = None;

    let server_config = config.clone();

    HttpServer::new(move || {
        let app_config = server_config.clone();
        let db_pool = connection_pool.clone();
        let mut app = App::new()
            .app_data(web::Data::new(last_frame.clone()))
            .app_data(web::Data::new(app_config.clone()))
            .app_data(web::Data::new(db_pool));

        // Add GPhoto camera as app data
        #[cfg(target_os = "linux")]
        {
            app = app.app_data(web::Data::new(gphoto_camera.clone()));
        }

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
    .run()
    .await
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::io::Result<()> {
    // Initialize tracing
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::error!("This binary is intended to run on Linux (Raspberry Pi). The V4L2-based preview and capture are Linux-only.");
    tracing::error!(
        "Build for the target device (e.g., aarch64-unknown-linux-gnu) and run it there."
    );
    Ok(())
}
