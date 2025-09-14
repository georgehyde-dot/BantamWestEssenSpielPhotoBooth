use actix_web::{post, web, HttpResponse, Responder};
use serde_json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info, warn};

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
    info!("Print photo request received with body: {:?}", body);

    let filename = match body.get("filename").and_then(|v| v.as_str()) {
        Some(f) => {
            info!("Filename from request: {}", f);
            f
        }
        None => {
            warn!("Print request missing filename");
            return HttpResponse::BadRequest().json(serde_json::json!({
                "ok": false,
                "error": "filename is required"
            }));
        }
    };

    // Get copies from request or from session
    let mut copies = body
        .get("copies")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as u32; // Limit to 10 copies max for safety

    // We'll update this from session data if available
    info!("Initial copies from request: {}", copies);

    // Validate filename for security
    if filename.contains('/') || filename.contains("..") {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "ok": false,
            "error": "Invalid filename"
        }));
    }

    let file_path = config.storage.base_path.join(filename);
    info!("Looking for file at: {:?}", file_path);
    info!("Base path is: {:?}", config.storage.base_path);

    // Check if file exists
    if !file_path.exists() {
        warn!("File not found at path: {:?}", file_path);
        // List files in the directory for debugging
        if let Ok(entries) = std::fs::read_dir(&config.storage.base_path) {
            info!("Files in base directory:");
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    info!("  - {}", name);
                }
            }
        }
        return HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": format!("File not found: {}", filename)
        }));
    }
    info!("File found successfully at: {:?}", file_path);

    // Get session data if session_id is provided
    let mut name_text = config.template.name_placeholder.clone();
    let mut headline_text = config.template.headline_placeholder.clone();
    let mut story_text = config.template.story_placeholder.clone();
    let mut session_to_update = None;

    if let Some(session_id) = body.get("session_id").and_then(|v| v.as_str()) {
        info!("Loading session {} for print job", session_id);
        match Session::load(session_id, &db_pool).await {
            Ok(Some(session)) => {
                info!("Session loaded successfully");
                info!("Session data - group_name: {:?}, headline: {:?}, story_text length: {:?}, copies: {:?}",
                    session.group_name,
                    session.headline,
                    session.story_text.as_ref().map(|s| s.len()),
                    session.copies_printed
                );

                // Use group_name for the name field
                if let Some(group_name) = &session.group_name {
                    name_text = group_name.clone();
                    info!("Using group_name: {}", name_text);
                }
                // Use session's headline if available
                if let Some(headline) = &session.headline {
                    headline_text = headline.clone();
                    info!("Using headline: {}", headline_text);
                }
                // Use session's story text if available
                if let Some(story) = &session.story_text {
                    story_text = story.clone();
                    info!("Using story text: {} chars", story_text.len());
                }
                // Get copies from session if not provided in request
                if body.get("copies").is_none() && session.copies_printed > 0 {
                    copies = session.copies_printed as u32;
                    info!("Using copies from session: {}", copies);
                }
                // Store session for later update
                session_to_update = Some(session);
            }
            Ok(None) => {
                warn!("Session {} not found when printing", session_id);
            }
            Err(e) => {
                warn!("Failed to load session {} for printing: {}", session_id, e);
            }
        }
    } else {
        info!("No session_id provided in print request");
    }

    // Create templated version of the photo
    let templated_filename = config
        .storage
        .base_path
        .join(format!("print_{}.png", chrono::Utc::now().timestamp()));

    info!("Creating templated print:");
    info!("  Source: {:?}", file_path);
    info!("  Destination: {:?}", templated_filename);
    info!("  Background: {:?}", config.background_path());
    info!("  Name text: '{}'", name_text);
    info!("  Headline: '{}'", headline_text);
    info!(
        "  Story text: '{}' (length: {})",
        if story_text.len() > 50 {
            format!("{}...", &story_text[..50])
        } else {
            story_text.clone()
        },
        story_text.len()
    );
    info!("  Header text: '{}'", config.template.header_text);

    match templates::create_templated_print_with_background(
        file_path.to_str().unwrap(),
        templated_filename.to_str().unwrap(),
        &config.template.header_text,
        &name_text,
        &headline_text,
        &story_text,
        config.background_path().to_str().unwrap(),
    ) {
        Ok(_) => {
            info!("Template created successfully");
            // Update session with templated print path if we have a session
            if let Some(mut session) = session_to_update {
                let templated_path = format!("print_{}.png", chrono::Utc::now().timestamp());
                session.photo_path = Some(templated_path);

                // Save the updated session
                if let Err(e) = session.update(&db_pool).await {
                    warn!("Failed to update session with templated print path: {}", e);
                }
            }

            // Use the templated file for printing
            let print_job = PrintJob {
                file_path: templated_filename.to_str().unwrap().to_string(),
                copies: copies,
                paper_size: PaperSize::Photo4x6,
                quality: PrintQuality::High,
            };

            info!("Sending print job to printer: {:?}", print_job);
            match printer.print_photo(print_job).await {
                Ok(job_id) => {
                    info!("Print job submitted successfully with ID: {}", job_id);
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
                    error!("Print job failed: {}", e);
                    // Clean up on error
                    let _ = std::fs::remove_file(&templated_filename);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "ok": false,
                        "error": format!("Print failed: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to create template: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "ok": false,
                "error": format!("Failed to create template: {}", e)
            }))
        }
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
