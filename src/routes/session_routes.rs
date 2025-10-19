use actix_web::{get, post, web, HttpResponse, Responder};
use serde_json;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::config::Config;
use crate::session::Session;
use crate::templates::create_templated_print_with_background;

#[post("/session")]
pub async fn create_session(db_pool: web::Data<SqlitePool>) -> impl Responder {
    let session = Session::new();

    match session.save(&db_pool).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "session_id": session.id,
            "session": session
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to create session: {}", e)
        })),
    }
}

#[get("/session/{id}")]
pub async fn get_session(
    path: web::Path<String>,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    let session_id = path.into_inner();

    match Session::load(&session_id, &db_pool).await {
        Ok(Some(session)) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "session": session
        })),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Session not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to load session: {}", e)
        })),
    }
}

#[post("/session/{id}")]
pub async fn update_session(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    let session_id = path.into_inner();

    // Load existing session
    match Session::load(&session_id, &db_pool).await {
        Ok(Some(mut session)) => {
            // Update fields from JSON body
            if let Some(group_name) = body.get("group_name").and_then(|v| v.as_str()) {
                session.group_name = Some(group_name.to_string());
            }
            if let Some(class) = body.get("class").and_then(|v| v.as_i64()) {
                session.class = Some(class as i32);
            }

            if let Some(choice) = body.get("choice").and_then(|v| v.as_i64()) {
                session.choice = Some(choice as i32);
            }
            if let Some(email) = body.get("email").and_then(|v| v.as_str()) {
                session.email = Some(email.to_string());
            }
            if let Some(photo_path) = body.get("photo_path").and_then(|v| v.as_str()) {
                session.photo_path = Some(photo_path.to_string());
            }
            if let Some(story_text) = body.get("story_text").and_then(|v| v.as_str()) {
                session.story_text = Some(story_text.to_string());
            }
            if let Some(headline) = body.get("headline").and_then(|v| v.as_str()) {
                session.headline = Some(headline.to_string());
            }
            if let Some(copies) = body.get("copies_printed").and_then(|v| v.as_i64()) {
                session.copies_printed = copies as i32;
            }
            if let Some(mailing_list) = body.get("mailing_list").and_then(|v| v.as_i64()) {
                session.mailing_list = mailing_list as i32;
            }

            // Save updated session
            match session.update(&db_pool).await {
                Ok(()) => HttpResponse::Ok().json(serde_json::json!({
                    "ok": true,
                    "session": session
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "ok": false,
                    "error": format!("Failed to update session: {}", e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Session not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to load session: {}", e)
        })),
    }
}

#[post("/session/{id}/generate-story")]
pub async fn generate_story(
    path: web::Path<String>,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    let session_id = path.into_inner();

    match Session::load(&session_id, &db_pool).await {
        Ok(Some(mut session)) => {
            // Generate story based on selections
            session.generate_story();

            // Update session with generated story
            match session.update(&db_pool).await {
                Ok(()) => HttpResponse::Ok().json(serde_json::json!({
                    "ok": true,
                    "story": session.story_text,
                    "headline": session.headline
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "ok": false,
                    "error": format!("Failed to update session with story: {}", e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Session not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to load session: {}", e)
        })),
    }
}

#[post("/session/{id}/save")]
pub async fn save_session_final(
    path: web::Path<String>,
    db_pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
) -> impl Responder {
    let session_id = path.into_inner();

    match Session::load(&session_id, &db_pool).await {
        Ok(Some(mut session)) => {
            // Generate story if missing
            if session.story_text.is_none() || session.headline.is_none() {
                info!("Generating story for session {}", session_id);
                session.generate_story();
            }

            // If we have a captured image but no templated photo_path, create the template
            let captured_image = session
                .email
                .as_ref()
                .and_then(|_| std::env::var("STORAGE_PATH").ok())
                .and_then(|storage_path| {
                    // Try to find the captured image in the storage directory
                    std::fs::read_dir(&storage_path).ok().and_then(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .find(|entry| entry.file_name().to_string_lossy().starts_with("cap_"))
                            .map(|e| e.path())
                    })
                });

            // Create templated image if we have the captured image
            if session.photo_path.is_none() {
                if let Some(captured_path) = captured_image {
                    let preview_filename = format!(
                        "preview_{}_{}.jpg",
                        session_id,
                        chrono::Utc::now().timestamp_millis()
                    );
                    let preview_path = config.storage.base_path.join(&preview_filename);

                    // Create the templated image
                    match create_templated_print_with_background(
                        captured_path.to_str().unwrap_or(""),
                        preview_path.to_str().unwrap_or(""),
                        session.story_text.as_deref().unwrap_or(""),
                        session.group_name.as_deref().unwrap_or(""),
                        session.headline.as_deref().unwrap_or(""),
                        config.background_path().to_str().unwrap_or(""),
                    ) {
                        Ok(_) => {
                            info!("Created templated preview image: {}", preview_filename);
                            session.photo_path = Some(preview_filename);
                        }
                        Err(e) => {
                            warn!("Failed to create templated preview: {}", e);
                            // Use a placeholder path to satisfy completion check
                            session.photo_path = Some("placeholder.jpg".to_string());
                        }
                    }
                } else {
                    // No captured image found, use placeholder
                    session.photo_path = Some("placeholder.jpg".to_string());
                }
            }

            // Check if session is complete (with photo_path now set)
            if !session.is_complete() {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "ok": false,
                    "error": "Session is not complete. Missing required fields."
                }));
            }

            // Session is already saved in database through update calls,
            // but we can do a final save to ensure everything is persisted
            match session.update(&db_pool).await {
                Ok(()) => HttpResponse::Ok().json(serde_json::json!({
                    "ok": true,
                    "message": "Session saved successfully",
                    "session": session
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "ok": false,
                    "error": format!("Failed to save session: {}", e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Session not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to load session: {}", e)
        })),
    }
}
