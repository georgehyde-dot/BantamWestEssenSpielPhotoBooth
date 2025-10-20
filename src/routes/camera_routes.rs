use actix_web::{get, post, web, HttpResponse, Responder};
use async_stream;
use bytes::Bytes;
use serde_json;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};

use tracing::{debug, error, info, warn};

use crate::config::Config;

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
        .stderr(std::process::Stdio::piped());

        info!("Spawning ffmpeg process for MJPEG stream from {}", v4l2_device);
        let mut process = match cmd.spawn() {
            Ok(p) => {
                info!("FFmpeg process started successfully, PID: {:?}", p.id());
                p
            },
            Err(e) => {
                error!("Failed to start ffmpeg for preview stream: {}", e);
                error!("Command was: ffmpeg -f v4l2 -video_size 1920x1080 -i {} -f mjpeg -q:v 5 -r 30 -", v4l2_device);
                return;
            }
        };

        let stdout = process.stdout.take().expect("Failed to get stdout");
        let stderr = process.stderr.take().expect("Failed to get stderr");

        // Spawn a task to log stderr output
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut stderr_reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                warn!("FFmpeg stderr: {}", line);
            }
        });

        let mut reader = tokio::io::BufReader::new(stdout);
        info!("Starting MJPEG stream parsing");

        // MJPEG stream parsing
        const JPEG_START: &[u8] = &[0xFF, 0xD8];
        const JPEG_END: &[u8] = &[0xFF, 0xD9];
        const BOUNDARY: &str = "frame";

        let mut buffer = Vec::with_capacity(1024 * 1024); // 1MB buffer
        let mut jpeg_buffer = Vec::new();
        let mut in_jpeg = false;
        let mut total_bytes = 0usize;
        let mut frame_count = 0u32;
        let start_time = std::time::Instant::now();

        use tokio::io::AsyncReadExt;

        loop {
            let mut chunk = vec![0u8; 65536]; // 64KB chunks
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    warn!("Preview stream ended");
                    break;
                }
                Ok(n) => {
                    total_bytes += n;
                    if total_bytes < 1000 {
                        debug!("Read {} bytes from stream (total: {})", n, total_bytes);
                    }
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
                                frame_count += 1;
                                if frame_count % 30 == 1 {  // Log every 30th frame
                                    let elapsed = start_time.elapsed();
                                    info!("Streaming: {} frames, {} bytes, {:.1} FPS",
                                         frame_count, total_bytes,
                                         frame_count as f32 / elapsed.as_secs_f32());
                                }

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
                    error!("Error reading preview stream: {}", e);
                    error!("Read {} bytes total before error", total_bytes);
                    break;
                }
            }
        }

        let _ = process.kill().await;
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "multipart/x-mixed-replace; boundary=frame"))
        .streaming(stream)
}

#[post("/capture")]
pub async fn capture_image(
    config: web::Data<Config>,
    _db_pool: web::Data<SqlitePool>,
    body: Option<web::Json<serde_json::Value>>,
    gphoto_camera: web::Data<Arc<Mutex<Option<Arc<crate::gphoto_camera::GPhotoCamera>>>>>,
) -> impl Responder {
    let capture_start = std::time::Instant::now();
    info!("=== CAPTURE IMAGE STARTED ===");
    info!("Request received at: {:?}", capture_start);
    info!("Storage base path: {:?}", config.storage.base_path);

    std::fs::create_dir_all(&config.storage.base_path).ok();
    info!(
        "Created/verified storage directory: {:?}",
        config.storage.base_path
    );

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

    info!("Capture request with session_id: {:?}", session_id);

    // Use GPhoto2 for high-resolution capture
    info!("Using GPhoto2 for high-resolution capture");

    let timestamp = chrono::Utc::now().timestamp();
    let filename = config
        .storage
        .base_path
        .join(format!("cap_{}.jpg", timestamp));

    info!("Will save captured photo to: {:?}", filename);

    let save_path = filename.clone();

    // Use the shared GPhoto2 camera instance
    let camera_opt = gphoto_camera.lock().unwrap().clone();
    info!("GPhoto camera available: {}", camera_opt.is_some());
    info!("Time since request start: {:?}", capture_start.elapsed());

    let capture_result = if let Some(camera) = camera_opt.clone() {
        info!("Starting photo capture via GPhoto2...");
        let gphoto_start = std::time::Instant::now();
        match camera.capture_photo(save_path.to_str().unwrap_or("")).await {
            Ok(jpeg_data) => {
                info!(
                    "Photo captured successfully, size: {} bytes, capture took: {:?}",
                    jpeg_data.len(),
                    gphoto_start.elapsed()
                );
                // Save the JPEG directly
                let save_path_log = save_path.clone();
                let save_start = std::time::Instant::now();
                let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
                    info!("Saving JPEG to disk: {:?}", save_path);
                    std::fs::write(&save_path, &jpeg_data)
                        .map_err(|e| format!("save JPEG: {e}"))?;
                    info!("JPEG saved successfully");
                    Ok(())
                });
                info!(
                    "JPEG save task spawned for: {:?}, save task spawn took: {:?}",
                    save_path_log,
                    save_start.elapsed()
                );

                // Restart the preview stream after capture
                info!("Restarting preview stream after capture");
                let preview_restart_start = std::time::Instant::now();

                // Start preview in background (simplified - no frame buffer needed)
                let camera_clone = camera.clone();
                tokio::spawn(async move {
                    if let Err(e) = camera_clone.start_preview_stream().await {
                        warn!("Failed to restart preview stream: {}", e);
                    } else {
                        info!("Preview stream restarted successfully");
                    }
                });
                info!(
                    "Preview restart task spawned in: {:?}",
                    preview_restart_start.elapsed()
                );

                Some((res, filename))
            }
            Err(e) => {
                error!(
                    "GPhoto2 capture failed after {:?}: {}",
                    gphoto_start.elapsed(),
                    e
                );
                error!("Total time since request: {:?}", capture_start.elapsed());

                // Try to restart preview even after failure
                info!("Attempting to restart preview stream after failed capture");

                let camera_clone = camera.clone();
                tokio::spawn(async move {
                    info!("Starting preview restart after failure...");
                    let restart_time = std::time::Instant::now();
                    if let Err(e) = camera_clone.start_preview_stream().await {
                        warn!(
                            "Failed to restart preview stream after {:?}: {}",
                            restart_time.elapsed(),
                            e
                        );
                    } else {
                        info!(
                            "Preview stream restarted successfully after failure in {:?}",
                            restart_time.elapsed()
                        );
                    }
                });

                None
            }
        }
    } else {
        error!("GPhoto2 camera not available - camera not initialized");
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

                    info!("Photo capture successful!");
                    info!("  - Filename: {}", file_name);
                    info!("  - Web path: {}", file_path);
                    info!("  - Full path: {:?}", filename);

                    // Update session if session_id was provided
                    let mut response_json = serde_json::json!({
                        "ok": true,
                        "path": file_path.clone(),
                        "file": file_name,
                        "redirect": format!("/photo?file={}", file_name),
                    });

                    if let Some(session_id) = session_id {
                        // Don't save the raw photo path - we'll save the templated version later
                        response_json["session_id"] = serde_json::json!(&session_id);
                        info!(
                            "Session {} will be updated with templated photo later",
                            session_id
                        );
                    }

                    info!("=== CAPTURE IMAGE COMPLETED SUCCESSFULLY ===");
                    info!("Total capture request time: {:?}", capture_start.elapsed());
                    HttpResponse::Ok().json(response_json)
                }
                Ok(Err(e)) => {
                    error!("Failed to save captured photo: {}", e);
                    error!(
                        "Total time before save failure: {:?}",
                        capture_start.elapsed()
                    );
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "ok": false, "error": e }))
                }
                Err(_e) => {
                    error!("Task join error while saving photo");
                    error!(
                        "Total time before join error: {:?}",
                        capture_start.elapsed()
                    );
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "ok": false, "error": "join error" }))
                }
            }
        }
        None => {
            error!("=== CAPTURE IMAGE FAILED ===");
            error!(
                "No camera available, total request time: {:?}",
                capture_start.elapsed()
            );
            HttpResponse::InternalServerError().json(serde_json::json!({
                "ok": false,
                "error": "camera capture failed"
            }))
        }
    }
}
