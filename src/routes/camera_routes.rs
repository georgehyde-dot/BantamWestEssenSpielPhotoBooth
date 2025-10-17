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

    // Check camera device type
    let camera_device_type =
        std::env::var("CAMERA_DEVICE_TYPE").unwrap_or_else(|_| "none".to_string());

    let stream = async_stream::stream! {
        // If no camera, return empty stream
        if camera_device_type == "none" || camera_device_type == "unknown" {
            info!("No camera available - returning empty preview stream");
            return;
        }

        info!("Starting direct preview stream from {} (device type: {})", v4l2_device, camera_device_type);

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

    // Check camera device type
    let camera_device_type =
        std::env::var("CAMERA_DEVICE_TYPE").unwrap_or_else(|_| "none".to_string());

    info!(
        "Capture: detected camera device type: {}",
        camera_device_type
    );

    let filename = config
        .storage
        .base_path
        .join(format!("cap_{}.jpg", chrono::Utc::now().timestamp()));

    let save_path = filename.clone();

    // Handle capture based on device type
    let capture_result = if camera_device_type != "loopback" && camera_device_type != "webcam" {
        // No camera available - create a placeholder image
        info!("No camera available, creating placeholder image");

        // Create a simple black JPEG as placeholder
        let placeholder_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
            // Create a minimal JPEG file (1x1 black pixel)
            // This is a valid minimal JPEG file
            let minimal_jpeg: Vec<u8> = vec![
                0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
                0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
                0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
                0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
                0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
                0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
                0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01,
                0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
                0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
                0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10,
                0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
                0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
                0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
                0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
                0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37,
                0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
                0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73,
                0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
                0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
                0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
                0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6,
                0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
                0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08,
                0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0xFB, 0xD2, 0x8A, 0x28, 0x03, 0xFF, 0xD9,
            ];

            std::fs::write(&save_path, &minimal_jpeg)
                .map_err(|e| format!("Failed to save placeholder image: {}", e))?;
            Ok(())
        });

        match placeholder_result.await {
            Ok(Ok(())) => {
                info!("No camera available - using placeholder image");
                Some((
                    tokio::task::spawn_blocking(move || -> Result<(), String> { Ok(()) }),
                    filename,
                ))
            }
            Ok(Err(e)) => {
                error!("Failed to create placeholder image: {}", e);
                None
            }
            Err(e) => {
                error!("Failed to create placeholder image task: {:?}", e);
                None
            }
        }
    } else if camera_device_type == "webcam" {
        // Capture directly from webcam
        info!("Using webcam for direct capture");
        let v4l2_device = config.camera.v4l2_loopback_device.clone();

        // Use ffmpeg to capture a frame from the webcam
        let output = tokio::process::Command::new("ffmpeg")
            .args(&[
                "-f",
                "v4l2",
                "-i",
                &v4l2_device,
                "-frames:v",
                "1",
                "-q:v",
                "2",  // High quality JPEG
                "-y", // Overwrite output
                save_path.to_str().unwrap_or(""),
            ])
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                info!("Webcam capture successful");
                // Read the saved file
                match tokio::fs::read(&save_path).await {
                    Ok(jpeg_data) => {
                        let res =
                            tokio::task::spawn_blocking(move || -> Result<(), String> { Ok(()) });
                        Some((res, filename))
                    }
                    Err(e) => {
                        warn!("Failed to read captured image: {}", e);
                        None
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Webcam capture failed: {}", stderr);
                None
            }
            Err(e) => {
                warn!("Failed to run webcam capture command: {}", e);
                // Fall back to placeholder image
                warn!("Creating placeholder image after webcam failure");
                let placeholder_jpeg: Vec<u8> = vec![
                    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01,
                    0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08,
                    0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A,
                    0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D,
                    0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22,
                    0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34,
                    0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0,
                    0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4,
                    0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                    0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01,
                    0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D,
                    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13,
                    0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
                    0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A,
                    0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35,
                    0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A,
                    0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67,
                    0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84,
                    0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98,
                    0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3,
                    0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,
                    0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1,
                    0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4,
                    0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00,
                    0x00, 0x3F, 0x00, 0xFB, 0xD2, 0x8A, 0x28, 0x03, 0xFF, 0xD9,
                ];
                if let Err(e) = std::fs::write(&save_path, &placeholder_jpeg) {
                    error!("Failed to save placeholder: {}", e);
                    None
                } else {
                    Some((
                        tokio::task::spawn_blocking(move || -> Result<(), String> { Ok(()) }),
                        filename,
                    ))
                }
            }
        }
    } else if camera_device_type == "loopback" {
        // Use GPhoto2 for high-resolution capture (Canon with loopback)
        info!("Using GPhoto2 for high-resolution capture (Canon camera)");

        // Use the shared GPhoto2 camera instance
        let camera_opt = gphoto_camera.lock().unwrap().clone();
        if let Some(camera) = camera_opt.clone() {
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
        }
    } else {
        // This shouldn't happen, but handle it as no camera
        warn!(
            "Unexpected camera device type: {} - using placeholder",
            camera_device_type
        );
        info!("Creating placeholder image for unexpected device type");

        // Create placeholder
        let placeholder_jpeg: Vec<u8> = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
            0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
            0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
            0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
            0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
            0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01,
            0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10,
            0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
            0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
            0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
            0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
            0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
            0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73,
            0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
            0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
            0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6,
            0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
            0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08,
            0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0xFB, 0xD2, 0x8A, 0x28, 0x03, 0xFF, 0xD9,
        ];

        if let Err(e) = std::fs::write(&save_path, &placeholder_jpeg) {
            error!("Failed to save placeholder: {}", e);
            None
        } else {
            Some((
                tokio::task::spawn_blocking(move || -> Result<(), String> { Ok(()) }),
                filename,
            ))
        }
    };

    // Handle the capture result
    match capture_result {
        Some((res, filename)) => {
            let res = res.await;

            match res {
                Ok(Ok(())) => {
                    let file_name = filename.file_name().unwrap().to_string_lossy();
                    let file_path = format!("/images/{}", file_name);

                    // Check if this was a placeholder due to no camera
                    let camera_device_type = std::env::var("CAMERA_DEVICE_TYPE")
                        .unwrap_or_else(|_| "loopback".to_string());
                    let is_placeholder =
                        camera_device_type == "none" || camera_device_type == "unknown";

                    // Update session if session_id was provided
                    let mut response_json = serde_json::json!({
                        "ok": true,
                        "path": file_path.clone(),
                        "file": file_name,
                        "redirect": format!("/photo?file={}", file_name),
                        "is_placeholder": is_placeholder,
                    });

                    if let Some(session_id) = session_id {
                        // Don't save the raw photo path - we'll save the templated version later
                        response_json["session_id"] = serde_json::json!(&session_id);
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
