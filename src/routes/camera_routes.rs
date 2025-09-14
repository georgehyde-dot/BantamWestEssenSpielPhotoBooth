use actix_web::{get, post, web, HttpResponse, Responder};
use async_stream;
use bytes::Bytes;
use serde_json;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};

use tracing::{info, warn};

use crate::config::Config;
use crate::session::Session;

#[get("/preview")]
pub async fn preview_stream(config: web::Data<Config>) -> impl Responder {
    let v4l2_device = config.camera.v4l2_loopback_device.clone();

    let stream = async_stream::stream! {
        info!("Starting direct preview stream from {}", v4l2_device);

        // Use ffmpeg to stream directly from v4l2 device as MJPEG
        let mut cmd = tokio::process::Command::new("ffmpeg");
        cmd.args(&[
            "-f", "v4l2",
            "-i", &v4l2_device,
            "-f", "mjpeg",
            "-q:v", "5",  // Quality setting (lower = better quality)
            "-r", "30",   // Frame rate
            "-"           // Output to stdout
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

        let mut process = match cmd.spawn() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to start ffmpeg for preview stream: {}", e);
                return;
            }
        };

        let stdout = process.stdout.take().expect("Failed to get stdout");
        let mut reader = tokio::io::BufReader::new(stdout);

        // MJPEG stream parsing
        const JPEG_START: &[u8] = &[0xFF, 0xD8];
        const JPEG_END: &[u8] = &[0xFF, 0xD9];
        const BOUNDARY: &str = "frame";

        let mut buffer = Vec::with_capacity(1024 * 1024); // 1MB buffer
        let mut jpeg_buffer = Vec::new();
        let mut in_jpeg = false;

        use tokio::io::AsyncReadExt;

        loop {
            let mut chunk = vec![0u8; 65536]; // 64KB chunks
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    warn!("Preview stream ended");
                    break;
                }
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);

                    // Look for JPEG markers
                    let mut i = 0;
                    while i < buffer.len() {
                        if !in_jpeg {
                            // Look for JPEG start
                            if i + 1 < buffer.len() && buffer[i] == JPEG_START[0] && buffer[i+1] == JPEG_START[1] {
                                in_jpeg = true;
                                jpeg_buffer.clear();
                                jpeg_buffer.push(buffer[i]);
                                jpeg_buffer.push(buffer[i+1]);
                                i += 2;
                            } else {
                                i += 1;
                            }
                        } else {
                            // Look for JPEG end
                            if i + 1 < buffer.len() && buffer[i] == JPEG_END[0] && buffer[i+1] == JPEG_END[1] {
                                jpeg_buffer.push(buffer[i]);
                                jpeg_buffer.push(buffer[i+1]);

                                // We have a complete JPEG frame
                                let boundary_prefix = format!("--{}\r\n", BOUNDARY).into_bytes();
                                let header = b"Content-Type: image/jpeg\r\n\r\n";
                                let tail = b"\r\n";

                                let mut part = Vec::with_capacity(
                                    boundary_prefix.len() + header.len() + jpeg_buffer.len() + tail.len()
                                );
                                part.extend_from_slice(&boundary_prefix);
                                part.extend_from_slice(header);
                                part.extend_from_slice(&jpeg_buffer);
                                part.extend_from_slice(tail);

                                yield Ok::<Bytes, actix_web::Error>(Bytes::from(part));

                                in_jpeg = false;
                                i += 2;
                            } else {
                                jpeg_buffer.push(buffer[i]);
                                i += 1;
                            }
                        }
                    }

                    // Keep unprocessed bytes
                    if in_jpeg {
                        buffer.clear();
                    } else {
                        buffer.drain(..i);
                    }
                }
                Err(e) => {
                    warn!("Error reading preview stream: {}", e);
                    break;
                }
            }
        }

        // Clean up process
        let _ = process.kill().await;
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "multipart/x-mixed-replace; boundary=frame"))
        .streaming(stream)
}

#[post("/capture")]
pub async fn capture_image(
    config: web::Data<Config>,
    db_pool: web::Data<SqlitePool>,
    body: Option<web::Json<serde_json::Value>>,
    gphoto_camera: web::Data<Arc<Mutex<Option<Arc<crate::gphoto_camera::GPhotoCamera>>>>>,
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

    // Extract session_id before any moves
    let session_id = body
        .as_ref()
        .and_then(|b| b.get("session_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Use GPhoto2 for high-resolution capture
    info!("Using GPhoto2 for high-resolution capture");

    let filename = config
        .storage
        .base_path
        .join(format!("cap_{}.jpg", chrono::Utc::now().timestamp()));

    let save_path = filename.clone();

    // Use the shared GPhoto2 camera instance
    let camera_opt = gphoto_camera.lock().unwrap().clone();
    let capture_result = if let Some(camera) = camera_opt.clone() {
        match camera.capture_photo(save_path.to_str().unwrap_or("")).await {
            Ok(jpeg_data) => {
                // Save the JPEG directly
                let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
                    std::fs::write(&save_path, &jpeg_data)
                        .map_err(|e| format!("save JPEG: {e}"))?;
                    Ok(())
                });

                // Restart the preview stream after capture
                info!("Restarting preview stream after capture");

                // Start preview in background (simplified - no frame buffer needed)
                let camera_clone = camera.clone();
                tokio::spawn(async move {
                    if let Err(e) = camera_clone.start_preview_stream().await {
                        warn!("Failed to restart preview stream: {}", e);
                    }
                });

                Some((res, filename))
            }
            Err(e) => {
                warn!("GPhoto2 capture failed: {}", e);

                // Try to restart preview even after failure
                info!("Attempting to restart preview stream after failed capture");

                let camera_clone = camera.clone();
                tokio::spawn(async move {
                    if let Err(e) = camera_clone.start_preview_stream().await {
                        warn!("Failed to restart preview stream: {}", e);
                    }
                });

                None
            }
        }
    } else {
        warn!("GPhoto2 camera not available - camera not initialized");
        None
    };

    // Handle the capture result
    match capture_result {
        Some((res, filename)) => {
            let res = res.await;

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

                    if let Some(session_id) = session_id {
                        // Load and update session
                        match Session::load(&session_id, &db_pool).await {
                            Ok(Some(mut session)) => {
                                if let Err(e) = session.set_photo_path(file_path, &db_pool).await {
                                    warn!("Failed to update session photo path: {}", e);
                                } else {
                                    response_json["session_id"] = serde_json::json!(&session_id);
                                }
                            }
                            Ok(None) => {
                                warn!(
                                    "Session {} not found when trying to associate photo",
                                    &session_id
                                );
                            }
                            Err(e) => {
                                warn!("Failed to load session {}: {}", &session_id, e);
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
            "error": "camera capture failed"
        })),
    }
}
