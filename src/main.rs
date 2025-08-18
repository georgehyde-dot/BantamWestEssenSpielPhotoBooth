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
use serde_json;
#[cfg(target_os = "linux")]
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
use image::{ImageBuffer, Rgb};
#[cfg(target_os = "linux")]
use std::io::Cursor;

// Module imports
mod printers;

#[cfg(target_os = "linux")]
use printers::{new_printer, PaperSize, PrintJob, PrintQuality, Printer, PrinterError};

// V4L2 camera functionality

#[cfg(target_os = "linux")]
fn video_settings() -> (String, u32, u32) {
    let dev = std::env::var("VIDEO_DEVICE").unwrap_or_else(|_| "/dev/video0".to_string());
    let width = std::env::var("VIDEO_WIDTH")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1920);
    let height = std::env::var("VIDEO_HEIGHT")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1080);
    (dev, width, height)
}

#[cfg(target_os = "linux")]
fn configure_device(dev: &mut Device, width: u32, height: u32) -> Result<Format, String> {
    // Capture trait in scope provides format() and set_format()
    let mut fmt = dev.format().map_err(|e| format!("format(): {e}"))?;
    fmt.width = width;
    fmt.height = height;

    // Try MJPEG first, fall back to YUYV
    fmt.fourcc = FourCC::new(b"MJPG");
    let fmt = dev
        .set_format(&fmt)
        .map_err(|e| format!("set_format(): {e}"))?;

    if fmt.fourcc == FourCC::new(b"MJPG") {
        return Ok(fmt);
    }

    Err(format!(
        "Device does not support MJPEG, got {}. Only MJPEG is supported.",
        fmt.fourcc
    ))
}

#[cfg(target_os = "linux")]
fn preview_loop(
    path: String,
    width: u32,
    height: u32,
    mut tx: mpsc::Sender<Vec<u8>>,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<(), String> {
    let mut dev = Device::with_path(path).map_err(|e| format!("open device: {e}"))?;
    let fmt = configure_device(&mut dev, width, height)?;

    let is_mjpeg = fmt.fourcc == FourCC::new(b"MJPG");
    let mut frame_count = 0;

    // Try userptr streaming first (better for HDMI capture devices)
    match try_userptr_streaming(
        &mut dev,
        &fmt,
        is_mjpeg,
        &mut tx,
        &mut frame_count,
        &last_frame,
    ) {
        Ok(()) => return Ok(()),
        Err(_) => Err(format!(
            "Device does not support userptr streaming, got {}. Only MJPEG is supported.",
            fmt.fourcc
        )),
    }
}

#[cfg(target_os = "linux")]
fn try_userptr_streaming(
    dev: &mut Device,
    fmt: &Format,
    is_mjpeg: bool,
    tx: &mut mpsc::Sender<Vec<u8>>,
    frame_count: &mut usize,
    last_frame: &Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<(), String> {
    let mut stream = userptr::Stream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create UserptrStream: {e}"))?;

    loop {
        match stream.next() {
            Ok((buffer, _meta)) => {
                *frame_count += 1;

                let jpeg_data = if is_mjpeg {
                    buffer.to_vec()
                } else {
                    continue;
                };

                {
                    let mut lf = last_frame.lock().unwrap();
                    *lf = Some(jpeg_data.clone());
                }
                if tx.blocking_send(jpeg_data).is_err() {
                    break;
                }
            }
            Err(e) => {
                return Err(format!("Userptr stream error: {e}"));
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let last_frame = Arc::new(Mutex::new(None::<Vec<u8>>));

    #[cfg(target_os = "linux")]
    let printer = match new_printer().await {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Failed to initialize printer: {}", e);
            None
        }
    };

    #[cfg(not(target_os = "linux"))]
    let printer = None;

    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(last_frame.clone()))
            .service(index)
            .service(preview_stream)
            .service(capture_image)
            .service(photo_page)
            .service(
                fs::Files::new("/images", "/usr/local/share/photo_booth").show_files_listing(),
            );

        if let Some(p) = printer.clone() {
            app = app.app_data(web::Data::new(p)).service(print_photo);
        }

        app
    })
    // Bind to all interfaces so you can reach it from other devices on the LAN
    .bind(("0.0.0.0", 8080))?
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
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(8);
    let (path, width, height) = video_settings();

    // Run V4L2 capture in a blocking thread, forward frames over channel
    let last_frame_arc = last_frame.get_ref().clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = preview_loop(path, width, height, tx, last_frame_arc) {
            eprintln!("Preview loop terminated: {e}");
        }
    });

    let stream = async_stream::stream! {
        // multipart/x-mixed-replace; boundary=frame
        const BOUNDARY: &str = "frame";
        let boundary_prefix = format!("--{}\r\n", BOUNDARY).into_bytes();
        let header = b"Content-Type: image/jpeg\r\n\r\n";
        let tail = b"\r\n";

        while let Some(frame) = rx.recv().await {
            let mut part = Vec::with_capacity(boundary_prefix.len() + header.len() + frame.len() + tail.len());
            part.extend_from_slice(&boundary_prefix);
            part.extend_from_slice(header);
            part.extend_from_slice(&frame);
            part.extend_from_slice(tail);

            // Convert to bytes and yield a Result<Bytes, actix_web::Error>
            yield Ok::<Bytes, actix_web::Error>(Bytes::from(part));
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "multipart/x-mixed-replace; boundary=frame"))
        // NOTE: no .no_chunking() call — streaming will chunk as appropriate
        .streaming(stream)
}

#[cfg(target_os = "linux")]
#[post("/capture")]
async fn capture_image(last_frame: web::Data<Arc<Mutex<Option<Vec<u8>>>>>) -> impl Responder {
    std::fs::create_dir_all("/usr/local/share/photo_booth").ok();

    // Set proper permissions on directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) =
            std::fs::metadata("/usr/local/share/photo_booth").and_then(|m| Ok(m.permissions()))
        {
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions("/usr/local/share/photo_booth", perms);
        }
    }
    let filename = format!(
        "/usr/local/share/photo_booth/cap_{}.png",
        chrono::Utc::now().timestamp()
    );

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
                    HttpResponse::Ok().json(serde_json::json!({
                        "ok": true,
                        "path": format!("/images/{}", std::path::Path::new(&filename).file_name().unwrap().to_string_lossy()),
                        "file": std::path::Path::new(&filename).file_name().unwrap().to_string_lossy(),
                        "redirect": format!("/photo?file={}", std::path::Path::new(&filename).file_name().unwrap().to_string_lossy()),
                    }))
                }
                Ok(Err(e)) => {
                    HttpResponse::InternalServerError().json(serde_json::json!({ "ok": false, "error": e }))
                }
                Err(_e) => {
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "ok": false, "error": "join error" }))
                }
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

    let file_path = format!("/usr/local/share/photo_booth/{}", filename);

    // Check if file exists
    if !std::path::Path::new(&file_path).exists() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Image file not found"
        }));
    }

    let print_job = PrintJob {
        file_path,
        copies: 1,
        paper_size: PaperSize::Photo4x6,
        quality: PrintQuality::High,
    };

    match printer.print_photo(print_job).await {
        Ok(job_id) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "job_id": job_id,
            "message": format!("Print job submitted successfully. Job ID: {}", job_id)
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Print failed: {}", e)
        })),
    }
}

#[cfg(target_os = "linux")]
fn try_userptr_capture(dev: &mut Device, fmt: &Format, is_mjpeg: bool) -> Result<Vec<u8>, String> {
    let mut stream = userptr::Stream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create userptr capture stream: {e}"))?;

    // Grab a few frames and keep the last
    let mut image: Option<Vec<u8>> = None;
    for attempt in 0..5 {
        match stream.next() {
            Ok((buffer, _meta)) => {
                let jpeg_data = if is_mjpeg { buffer.to_vec() } else { continue };
                image = Some(jpeg_data);

                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            Err(e) => {
                return Err(format!(
                    "userptr capture error on attempt {}: {}",
                    attempt + 1,
                    e
                ));
            }
        }
    }
    image.ok_or_else(|| "No frame captured after 5 attempts".to_string())
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::io::Result<()> {
    eprintln!(
        "This binary is intended to run on Linux (Raspberry Pi). The V4L2-based preview and capture are Linux-only."
    );
    eprintln!("Build for the target device (e.g., aarch64-unknown-linux-gnu) and run it there.");
    Ok(())
}
