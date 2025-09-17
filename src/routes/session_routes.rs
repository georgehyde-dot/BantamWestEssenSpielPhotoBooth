use actix_web::{get, post, web, HttpResponse, Responder};
use serde_json;
use sqlx::SqlitePool;

use crate::session::Session;

#[derive(serde::Serialize, sqlx::FromRow)]
struct WallPhoto {
    id: String,
    photo_path: String,
    group_name: Option<String>,
    headline: Option<String>,
    created_at: String,
}

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
            if let Some(weapon) = body.get("weapon").and_then(|v| v.as_i64()) {
                session.weapon = Some(weapon as i32);
            }
            if let Some(land) = body.get("land").and_then(|v| v.as_i64()) {
                session.land = Some(land as i32);
            }
            if let Some(companion) = body.get("companion").and_then(|v| v.as_i64()) {
                session.companion = Some(companion as i32);
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
) -> impl Responder {
    let session_id = path.into_inner();

    match Session::load(&session_id, &db_pool).await {
        Ok(Some(session)) => {
            // Check if session is complete
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

#[get("/sessions/recent")]
pub async fn get_recent_sessions(db_pool: web::Data<SqlitePool>) -> impl Responder {
    // Fetch recent sessions that have templated photos
    let query = r#"
        SELECT id, photo_path, group_name, headline, created_at
        FROM session
        WHERE photo_path IS NOT NULL
        AND (photo_path LIKE 'print_%' OR photo_path LIKE 'preview_%')
        ORDER BY created_at DESC
        LIMIT 50
    "#;

    match sqlx::query_as::<_, WallPhoto>(query)
        .fetch_all(db_pool.as_ref())
        .await
    {
        Ok(photos) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "photos": photos
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to fetch recent sessions: {}", e)
        })),
    }
}
