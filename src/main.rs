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
mod printers;
mod routes;
mod session;
mod templates;

#[cfg(target_os = "linux")]
use printers::{new_printer, PaperSize, PrintJob, PrintQuality, Printer, PrinterError};
#[cfg(target_os = "linux")]
use routes::{create_session, get_session, update_session};
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
            .service(index)
            .service(preview_stream)
            .service(capture_image)
            .service(photo_page)
            .service(create_session)
            .service(get_session)
            .service(update_session)
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

// Embed HTML files at compile time
#[cfg(target_os = "linux")]
const INDEX_HTML: &str = include_str!("../html/index.html");
#[cfg(target_os = "linux")]
const PHOTO_HTML: &str = include_str!("../html/photo.html");

#[cfg(target_os = "linux")]
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

#[cfg(target_os = "linux")]
#[get("/photo")]
async fn photo_page(
    _query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(PHOTO_HTML)
}

#[cfg(target_os = "linux")]
#[get("/preview")]
async fn preview_stream(last_frame: web::Data<Arc<Mutex<Option<Vec<u8>>>>>) -> impl Responder {
    let last_frame_arc = last_frame.get_ref().clone();

    let stream = async_stream::stream! {
        // multipart/x-mixed-replace; boundary=frame
        const BOUNDARY: &str = "frame";
        let boundary_prefix = format!("--{}\r\n", BOUNDARY).into_bytes();
        let header = b"Content-Type: image/jpeg\r\n\r\n";
        let tail = b"\r\n";

        // Stream frames at ~30fps
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(33));

        loop {
            interval.tick().await;

            // Get the latest frame from the shared buffer
            let frame_opt = {
                last_frame_arc.lock().unwrap().clone()
            };

            if let Some(frame) = frame_opt {
                let mut part = Vec::with_capacity(boundary_prefix.len() + header.len() + frame.len() + tail.len());
                part.extend_from_slice(&boundary_prefix);
                part.extend_from_slice(header);
                part.extend_from_slice(&frame);
                part.extend_from_slice(tail);

                // Convert to bytes and yield a Result<Bytes, actix_web::Error>
                yield Ok::<Bytes, actix_web::Error>(Bytes::from(part));
            } else {
                // Log once if no frames are available
                static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
                    warn!("No frames available in preview stream");
                    LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "multipart/x-mixed-replace; boundary=frame"))
        .streaming(stream)
}

#[cfg(target_os = "linux")]
#[post("/capture")]
async fn capture_image(
    last_frame: web::Data<Arc<Mutex<Option<Vec<u8>>>>>,
    config: web::Data<Config>,
    db_pool: web::Data<SqlitePool>,
    body: Option<web::Json<serde_json::Value>>,
) -> impl Responder {
    std::fs::create_dir_all(&config.storage.base_path).ok();

    // Set proper permissions on directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) =
            std::fs::metadata(&config.storage.base_path).and_then(|m| Ok(m.permissions()))
        {
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&config.storage.base_path, perms);
        }
    }
    let filename = config
        .storage
        .base_path
        .join(format!("cap_{}.png", chrono::Utc::now().timestamp()));

    let img_opt = { last_frame.lock().unwrap().clone() };
    match img_opt {
        Some(bytes) => {
            let save_path = filename.clone();
            let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
                // Convert JPEG bytes to PNG format
                let img =
                    image::load_from_memory(&bytes).map_err(|e| format!("decode image: {e}"))?;
                img.save(&save_path).map_err(|e| format!("save PNG: {e}"))?;

                Ok(())
            })
            .await;

            match res {
                Ok(Ok(())) => {
                    let file_name = filename.file_name().unwrap().to_string_lossy();
                    let file_path = format!("/images/{}", file_name);

                    // Update session if session_id was provided
                    let mut response_json = serde_json::json!({
                        "ok": true,
                        "path": file_path.clone(),
                        "file": file_name,
                        "redirect": format!("/photo?file={}", file_name),
                    });

                    if let Some(body) = body {
                        if let Some(session_id) = body.get("session_id").and_then(|v| v.as_str()) {
                            // Load and update session
                            match Session::load(session_id, &db_pool).await {
                                Ok(Some(mut session)) => {
                                    if let Err(e) =
                                        session.set_photo_path(file_path, &db_pool).await
                                    {
                                        warn!("Failed to update session photo path: {}", e);
                                    } else {
                                        response_json["session_id"] = serde_json::json!(session_id);
                                    }
                                }
                                Ok(None) => {
                                    warn!(
                                        "Session {} not found when trying to associate photo",
                                        session_id
                                    );
                                }
                                Err(e) => {
                                    warn!("Failed to load session {}: {}", session_id, e);
                                }
                            }
                        }
                    }

                    HttpResponse::Ok().json(response_json)
                }
                Ok(Err(e)) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "ok": false, "error": e })),
                Err(_e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "ok": false, "error": "join error" })),
            }
        }
        None => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": "no frame available yet"
        })),
    }
}

#[cfg(target_os = "linux")]
#[post("/print")]
async fn print_photo(
    printer: web::Data<Arc<dyn Printer + Send + Sync>>,
    body: web::Json<serde_json::Value>,
    config: web::Data<Config>,
) -> impl Responder {
    let filename = match body.get("filename").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "ok": false,
                "error": "filename is required"
            }));
        }
    };

    // Validate filename for security
    if filename.contains('/') || filename.contains("..") {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "ok": false,
            "error": "Invalid filename"
        }));
    }

    let file_path = config.storage.base_path.join(filename);

    // Check if file exists
    if !file_path.exists() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Image file not found"
        }));
    }

    // Create templated version of the photo
    let templated_filename = config
        .storage
        .base_path
        .join(format!("print_{}.png", chrono::Utc::now().timestamp()));

    match templates::create_templated_print_with_background(
        file_path.to_str().unwrap(),
        templated_filename.to_str().unwrap(),
        &config.template.header_text,
        &config.template.name_placeholder,
        &config.template.headline_placeholder,
        &config.template.story_placeholder,
        config.background_path().to_str().unwrap(),
    ) {
        Ok(()) => {
            // Use the templated file for printing
            let print_job = PrintJob {
                file_path: templated_filename.to_str().unwrap().to_string(),
                copies: 1,
                paper_size: PaperSize::Photo4x6,
                quality: PrintQuality::High,
            };

            match printer.print_photo(print_job).await {
                Ok(job_id) => {
                    // Clean up templated file after sending to printer
                    tokio::task::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        let _ = std::fs::remove_file(&templated_filename);
                    });

                    HttpResponse::Ok().json(serde_json::json!({
                        "ok": true,
                        "job_id": job_id,
                        "message": format!("Print job submitted successfully. Job ID: {}", job_id)
                    }))
                }
                Err(e) => {
                    // Clean up on error
                    let _ = std::fs::remove_file(&templated_filename);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "ok": false,
                        "error": format!("Print failed: {}", e)
                    }))
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to create templated print: {}", e)
        })),
    }
}

#[cfg(target_os = "linux")]
#[post("/preview")]
async fn preview_print(
    body: web::Json<serde_json::Value>,
    config: web::Data<Config>,
) -> impl Responder {
    let filename = match body.get("filename").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "ok": false,
                "error": "filename is required"
            }));
        }
    };

    let file_path = config.storage.base_path.join(filename);

    // Check if file exists
    if !file_path.exists() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Image file not found"
        }));
    }

    // Create templated preview
    let preview_filename = format!("preview_{}.png", chrono::Utc::now().timestamp());
    let preview_path = config.storage.base_path.join(&preview_filename);

    match templates::create_templated_print_with_background(
        file_path.to_str().unwrap(),
        preview_path.to_str().unwrap(),
        &config.template.header_text,
        &config.template.name_placeholder,
        &config.template.headline_placeholder,
        &config.template.story_placeholder,
        config.background_path().to_str().unwrap(),
    ) {
        Ok(()) => {
            // Return the URL to the preview
            HttpResponse::Ok().json(serde_json::json!({
                "ok": true,
                "preview_url": format!("/images/{}", preview_filename)
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to create preview: {}", e)
        })),
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::io::Result<()> {
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
