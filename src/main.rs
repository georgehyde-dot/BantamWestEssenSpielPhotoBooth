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
mod image_processing;
mod printers;
mod routes;
mod session;
mod templates;

#[cfg(target_os = "linux")]
use printers::{new_printer, PaperSize, PrintJob, PrintQuality, Printer, PrinterError};
#[cfg(target_os = "linux")]
use routes::{
    camera_page, capture_image, companion_page, create_session, get_session, land_page,
    name_entry_page, photo_page, preview_print, preview_stream, print_photo, start_page,
    update_session, weapon_page,
};
#[cfg(target_os = "linux")]
use session::Session;
#[cfg(target_os = "linux")]
use tracing::{error, info, warn};

#[cfg(target_os = "linux")]
fn spawn_camera(
    config: config::CameraConfig,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
) -> tokio::task::JoinHandle<()> {
    info!("Spawning camera task for device: {}", config.device);
    tokio::spawn(async move {
        let camera = camera::Camera::new(config.clone());
        let (tx, mut rx) = mpsc::channel(10);

        // Start camera stream
        let camera_handle = tokio::spawn(async move {
            info!("Starting camera preview stream");
            if let Err(e) = camera.start_preview_stream(tx, last_frame.clone()).await {
                error!("Camera stream error: {}", e);
            }
        });

        // Drain the channel to prevent backpressure
        let mut frame_count = 0;
        while let Some(_frame) = rx.recv().await {
            frame_count += 1;
            if frame_count % 1000 == 0 {
                info!("Received {} frames from camera", frame_count);
            }
            // Frames are already stored in last_frame by the camera
        }

        warn!("Camera frame receiver loop ended");
        let _ = camera_handle.await;
    })
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
    let camera = {
        info!("Initializing camera with config: {:?}", config.camera);
        Some(spawn_camera(config.camera.clone(), last_frame.clone()))
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
            .app_data(web::Data::new(db_pool))
            .service(start_page)
            .service(name_entry_page)
            .service(camera_page)
            .service(preview_stream)
            .service(capture_image)
            .service(photo_page)
            .service(create_session)
            .service(get_session)
            .service(update_session)
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
