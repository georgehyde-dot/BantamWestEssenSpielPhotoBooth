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
    info!("=== PRINT PHOTO STARTED ===");
    info!("Print photo request received with body: {:?}", body);
    info!("Storage base path: {:?}", config.storage.base_path);

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
    info!("=== FILE MANAGEMENT - PRINT REQUEST ===");
    info!("  Raw photo filename: {}", filename);
    info!("  Raw photo full path: {:?}", file_path);
    info!("  Base storage path: {:?}", config.storage.base_path);

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
    info!("Photo file found successfully at: {:?}", file_path);

    // Get session data if session_id is provided
    let mut story_text = config.template.story_placeholder.clone();
    let mut group_name = String::new();
    let mut headline = String::new();
    let mut session_to_update = None;

    if let Some(session_id) = body.get("session_id").and_then(|v| v.as_str()) {
        info!("Loading session {} for print job", session_id);
        info!("Session ID provided: {}", session_id);
        match Session::load(session_id, &db_pool).await {
            Ok(Some(mut session)) => {
                info!("Session loaded successfully");

                // Generate story if missing
                if session.story_text.is_none() || session.headline.is_none() {
                    info!(
                        "Story/headline missing, generating for session {}",
                        session_id
                    );
                    session.generate_story();
                    // Save the generated story back to the session immediately
                    if let Err(e) = session.update(&db_pool).await {
                        warn!("Failed to update session with generated story: {}", e);
                    }
                }

                info!("Session data - group_name: {:?}, headline: {:?}, story_text length: {:?}, copies: {:?}",
                    session.group_name,
                    session.headline,
                    session.story_text.as_ref().map(|s| s.len()),
                    session.copies_printed
                );

                // Use session's story text if available
                if let Some(story) = &session.story_text {
                    story_text = story.clone();
                    info!("Using story text: {} chars", story_text.len());
                }

                // Extract group name
                if let Some(name) = &session.group_name {
                    group_name = name.clone();
                    info!("Using group name: {}", group_name);
                }

                // Extract headline
                if let Some(head) = &session.headline {
                    headline = head.clone();
                    info!("Using headline: {}", headline);
                }
                // Get copies from session if not provided in request
                if body.get("copies").is_none() && session.copies_printed > 0 {
                    copies = session.copies_printed as u32;
                    info!("Using copies from session: {}", copies);
                }
                // Store session for later update with templated path
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
    let timestamp = chrono::Utc::now().timestamp();
    let templated_filename_only = format!("print_{}.png", timestamp);
    let templated_filename = config.storage.base_path.join(&templated_filename_only);

    info!("=== FILE MANAGEMENT - CREATING TEMPLATE ===");
    info!("  Raw photo (INPUT): {:?}", file_path);
    info!("  Template (OUTPUT): {:?}", templated_filename);
    info!("  Template will be KEPT for display");
    info!("  Raw photo will be DELETED after 30 seconds");

    info!("=== CREATING TEMPLATED PRINT ===");
    info!("  Timestamp: {}", timestamp);
    info!("  Raw photo source: {:?}", file_path);
    info!("  Template destination: {:?}", templated_filename);
    info!("  Background image: {:?}", config.background_path());
    info!("  Group name: '{}'", group_name);
    info!("  Headline: '{}'", headline);
    info!(
        "  Story text: '{}' (length: {})",
        if story_text.len() > 50 {
            format!("{}...", &story_text[..50])
        } else {
            story_text.clone()
        },
        story_text.len()
    );

    match templates::create_templated_print_with_background(
        file_path.to_str().unwrap(),
        templated_filename.to_str().unwrap(),
        &story_text,
        &group_name,
        &headline,
        config.background_path().to_str().unwrap(),
    ) {
        Ok(_) => {
            info!("=== TEMPLATE CREATED SUCCESSFULLY ===");
            info!("  Template filename: {}", templated_filename_only);
            info!("  Template full path: {:?}", templated_filename);

            // IMPORTANT: Update session with templated print path BEFORE printing
            // This ensures the thank you page shows the correct templated image
            if let Some(mut session) = session_to_update {
                info!("=== UPDATING SESSION WITH TEMPLATED PATH ===");
                info!("  Session ID: {}", session.id);
                info!("  Old photo_path: {:?}", session.photo_path);
                info!("  New photo_path: {}", templated_filename_only);
                session.photo_path = Some(templated_filename_only.clone());

                // Save the updated session immediately
                match session.update(&db_pool).await {
                    Ok(_) => {
                        info!(
                            "Session {} updated successfully with templated path",
                            session.id
                        );
                        info!(
                            "Session now has templated photo: {}",
                            templated_filename_only
                        );
                        info!("=== FILE STATUS ===");
                        info!("  Raw photo: {} (will be deleted after print)", filename);
                        info!(
                            "  Template: {} (permanent, for display)",
                            templated_filename_only
                        );
                    }
                    Err(e) => {
                        error!(
                            "FAILED to update session {} with templated path: {}",
                            session.id, e
                        );
                        // Continue with print even if session update fails
                    }
                }
            } else {
                warn!("No session to update with templated path");
            }

            // Use the templated file for printing
            let print_job = PrintJob {
                file_path: templated_filename.to_str().unwrap().to_string(),
                copies: copies,
                paper_size: PaperSize::Photo4x6,
                quality: PrintQuality::Draft,
            };

            info!("Sending print job to printer: {:?}", print_job);
            match printer.print_photo(print_job).await {
                Ok(job_id) => {
                    info!("Print job submitted successfully with ID: {}", job_id);
                    info!("=== FILE MANAGEMENT - POST-PRINT CLEANUP SCHEDULED ===");
                    info!("  Template file (KEEPING): {:?}", templated_filename);
                    info!("  Raw file (DELETING in 30s): {:?}", file_path);
                    // Clean up raw capture file after sending to printer (keep the templated version)
                    let raw_file_to_delete = file_path.clone();
                    tokio::task::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                        info!("=== FILE MANAGEMENT - CLEANUP TIMER EXPIRED ===");
                        // Safety check: only delete raw capture files (cap_*.jpg)
                        if let Some(file_name) = raw_file_to_delete.file_name() {
                            let name_str = file_name.to_string_lossy();
                            if name_str.starts_with("cap_") && name_str.ends_with(".jpg") {
                                info!("Deleting raw capture file: {:?}", raw_file_to_delete);
                                match std::fs::remove_file(&raw_file_to_delete) {
                                    Ok(_) => info!(
                                        "Successfully deleted raw capture: {:?}",
                                        raw_file_to_delete
                                    ),
                                    Err(e) => warn!(
                                        "Failed to delete raw capture: {:?}, error: {}",
                                        raw_file_to_delete, e
                                    ),
                                }
                            } else {
                                warn!(
                                    "SAFETY CHECK: Refusing to delete non-capture file: {:?}",
                                    raw_file_to_delete
                                );
                                warn!("Only cap_*.jpg files should be deleted, not: {}", name_str);
                            }
                        } else {
                            warn!(
                                "Could not determine filename for cleanup: {:?}",
                                raw_file_to_delete
                            );
                        }
                    });

                    HttpResponse::Ok().json(serde_json::json!({
                        "ok": true,
                        "job_id": job_id,
                        "message": format!("Print job submitted successfully. Job ID: {}", job_id)
                    }))
                }
                Err(e) => {
                    error!("=== PRINT JOB FAILED ===");
                    error!("Print error: {}", e);
                    // Keep the templated file even on print failure - it's needed for the thank you page
                    info!(
                        "Keeping templated file despite print failure: {:?}",
                        templated_filename
                    );
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "ok": false,
                        "error": format!("Print failed: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("=== TEMPLATE CREATION FAILED ===");
            error!("Template error: {}", e);
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
    info!("Looking for photo at: {:?}", file_path);

    // Check if file exists
    if !file_path.exists() {
        error!("Photo file not found at: {:?}", file_path);
        return HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Image file not found"
        }));
    }

    // Get session data if session_id is provided
    let mut story_text = config.template.story_placeholder.clone();
    let mut group_name = String::new();
    let mut headline = String::new();

    if let Some(session_id) = body.get("session_id").and_then(|v| v.as_str()) {
        match Session::load(session_id, &db_pool).await {
            Ok(Some(session)) => {
                // Use session's story text if available
                if let Some(story) = &session.story_text {
                    story_text = story.clone();
                }
                // Use session's group name if available
                if let Some(name) = &session.group_name {
                    group_name = name.clone();
                }
                // Use session's headline if available
                if let Some(head) = &session.headline {
                    headline = head.clone();
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
        &story_text,
        &group_name,
        &headline,
        config.background_path().to_str().unwrap(),
    ) {
        Ok(()) => {
            // Update session with templated preview path if we have a session
            if let Some(session_id_str) = body.get("session_id").and_then(|v| v.as_str()) {
                match Session::load(session_id_str, &db_pool).await {
                    Ok(Some(mut session)) => {
                        session.photo_path = Some(preview_filename.clone());
                        if let Err(e) = session.update(&db_pool).await {
                            warn!(
                                "Failed to update session with templated preview path: {}",
                                e
                            );
                        }
                    }
                    Ok(None) => {
                        warn!(
                            "Session {} not found when updating preview path",
                            session_id_str
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load session {} for preview path update: {}",
                            session_id_str, e
                        );
                    }
                }
            }

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
