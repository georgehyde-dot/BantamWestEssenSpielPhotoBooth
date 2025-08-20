use actix_web::{post, web, HttpResponse, Responder};
use serde_json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::warn;

use crate::config::Config;
use crate::printers::{PaperSize, PrintJob, PrintQuality, Printer};
use crate::session::Session;
use crate::templates;

#[post("/print")]
pub async fn print_photo(
    printer: web::Data<Arc<dyn Printer + Send + Sync>>,
    body: web::Json<serde_json::Value>,
    config: web::Data<Config>,
    db_pool: web::Data<SqlitePool>,
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

    // Get session data if session_id is provided
    let mut name_text = config.template.name_placeholder.clone();
    let mut headline_text = config.template.headline_placeholder.clone();
    let mut story_text = config.template.story_placeholder.clone();

    if let Some(session_id) = body.get("session_id").and_then(|v| v.as_str()) {
        match Session::load(session_id, &db_pool).await {
            Ok(Some(session)) => {
                // Use group_name for the name field
                if let Some(group_name) = &session.group_name {
                    name_text = group_name.clone();
                }
                // Use session's headline if available
                if let Some(headline) = &session.headline {
                    headline_text = headline.clone();
                }
                // Use session's story text if available
                if let Some(story) = &session.story_text {
                    story_text = story.clone();
                }
            }
            Ok(None) => {
                warn!("Session {} not found when printing", session_id);
            }
            Err(e) => {
                warn!("Failed to load session {} for printing: {}", session_id, e);
            }
        }
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
        &name_text,
        &headline_text,
        &story_text,
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

#[post("/preview")]
pub async fn preview_print(
    body: web::Json<serde_json::Value>,
    config: web::Data<Config>,
    db_pool: web::Data<SqlitePool>,
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

    // Get session data if session_id is provided
    let mut name_text = config.template.name_placeholder.clone();
    let mut headline_text = config.template.headline_placeholder.clone();
    let mut story_text = config.template.story_placeholder.clone();

    if let Some(session_id) = body.get("session_id").and_then(|v| v.as_str()) {
        match Session::load(session_id, &db_pool).await {
            Ok(Some(session)) => {
                // Use group_name for the name field
                if let Some(group_name) = &session.group_name {
                    name_text = group_name.clone();
                }
                // Use session's headline if available
                if let Some(headline) = &session.headline {
                    headline_text = headline.clone();
                }
                // Use session's story text if available
                if let Some(story) = &session.story_text {
                    story_text = story.clone();
                }
            }
            Ok(None) => {
                warn!("Session {} not found when previewing", session_id);
            }
            Err(e) => {
                warn!("Failed to load session {} for preview: {}", session_id, e);
            }
        }
    }

    // Create templated preview
    let preview_filename = format!("preview_{}.png", chrono::Utc::now().timestamp());
    let preview_path = config.storage.base_path.join(&preview_filename);

    match templates::create_templated_print_with_background(
        file_path.to_str().unwrap(),
        preview_path.to_str().unwrap(),
        &config.template.header_text,
        &name_text,
        &headline_text,
        &story_text,
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
