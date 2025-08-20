use actix_web::{get, post, web, HttpResponse, Responder};
use async_stream;
use bytes::Bytes;
use serde_json;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::warn;

use crate::config::Config;
use crate::session::Session;

#[get("/preview")]
pub async fn preview_stream(last_frame: web::Data<Arc<Mutex<Option<Vec<u8>>>>>) -> impl Responder {
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

#[post("/capture")]
pub async fn capture_image(
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
