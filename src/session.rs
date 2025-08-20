use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::errors::{AppResult, DatabaseError};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub group_name: Option<String>,
    pub created_at: String,
    pub weapon: Option<i32>,
    pub land: Option<i32>,
    pub companion: Option<i32>,
    pub email: Option<String>,
    pub photo_path: Option<String>,
    pub copies_printed: i32,
    pub story_text: Option<String>,
    pub headline: Option<String>,
}

impl Session {
    /// Create a new session with a unique ID and current timestamp
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            group_name: None,
            created_at: Utc::now().to_rfc3339(),
            weapon: None,
            land: None,
            companion: None,
            email: None,
            photo_path: None,
            copies_printed: 0,
            story_text: None,
            headline: None,
        }
    }

    /// Save the session to the database
    pub async fn save(&self, pool: &SqlitePool) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO session (
                id, group_name, created_at, weapon, land, companion,
                email, photo_path, copies_printed, story_text, headline
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
            "#,
        )
        .bind(&self.id)
        .bind(&self.group_name)
        .bind(&self.created_at)
        .bind(self.weapon)
        .bind(self.land)
        .bind(self.companion)
        .bind(&self.email)
        .bind(&self.photo_path)
        .bind(self.copies_printed)
        .bind(&self.story_text)
        .bind(&self.headline)
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed(format!("Failed to save session: {}", e)))?;

        Ok(())
    }

    /// Update an existing session in the database
    pub async fn update(&self, pool: &SqlitePool) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE session SET
                group_name = ?2,
                weapon = ?3,
                land = ?4,
                companion = ?5,
                email = ?6,
                photo_path = ?7,
                copies_printed = ?8,
                story_text = ?9,
                headline = ?10
            WHERE id = ?1
            "#,
        )
        .bind(&self.id)
        .bind(&self.group_name)
        .bind(self.weapon)
        .bind(self.land)
        .bind(self.companion)
        .bind(&self.email)
        .bind(&self.photo_path)
        .bind(self.copies_printed)
        .bind(&self.story_text)
        .bind(&self.headline)
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed(format!("Failed to update session: {}", e)))?;

        Ok(())
    }

    /// Load a session by ID from the database
    pub async fn load(id: &str, pool: &SqlitePool) -> AppResult<Option<Self>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, group_name, created_at, weapon, land, companion,
                email, photo_path, copies_printed, story_text, headline
            FROM session
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed(format!("Failed to load session: {}", e)))?;

        Ok(session)
    }

    /// Get all sessions created within a time range
    pub async fn find_by_date_range(
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        pool: &SqlitePool,
    ) -> AppResult<Vec<Self>> {
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();

        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, group_name, created_at, weapon, land, companion,
                email, photo_path, copies_printed, story_text, headline
            FROM session
            WHERE created_at >= ?1 AND created_at <= ?2
            ORDER BY created_at DESC
            "#,
        )
        .bind(&start_str)
        .bind(&end_str)
        .fetch_all(pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed(format!("Failed to find sessions: {}", e)))?;

        Ok(sessions)
    }

    /// Get sessions by email
    pub async fn find_by_email(email: &str, pool: &SqlitePool) -> AppResult<Vec<Self>> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, group_name, created_at, weapon, land, companion,
                email, photo_path, copies_printed, story_text, headline
            FROM session
            WHERE email = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(email)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            DatabaseError::QueryFailed(format!("Failed to find sessions by email: {}", e))
        })?;

        Ok(sessions)
    }

    /// Set the photo path for this session
    pub async fn set_photo_path(&mut self, path: String, pool: &SqlitePool) -> AppResult<()> {
        self.photo_path = Some(path.clone());

        sqlx::query(
            r#"
            UPDATE session SET photo_path = ?2
            WHERE id = ?1
            "#,
        )
        .bind(&self.id)
        .bind(&path)
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed(format!("Failed to set photo path: {}", e)))?;

        Ok(())
    }

    /// Check if session has all required fields filled
    pub fn is_complete(&self) -> bool {
        self.group_name.is_some()
            && self.weapon.is_some()
            && self.land.is_some()
            && self.companion.is_some()
            && self.email.is_some()
            && self.photo_path.is_some()
            && self.story_text.is_some()
            && self.headline.is_some()
    }

    /// Generate story and headline based on selections
    pub fn generate_story(&mut self) {
        // This is a placeholder - you can implement the actual story generation logic
        // based on weapon, land, and companion selections
        if let (Some(weapon), Some(land), Some(companion)) =
            (self.weapon, self.land, self.companion)
        {
            self.headline = Some(format!("Adventure #{}", &self.id[..8]));
            self.story_text = Some(format!(
                "A brave adventurer wielding weapon {} journeyed through land {} alongside companion {}.",
                weapon, land, companion
            ));
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = Session::new();
        assert!(!session.id.is_empty());
        assert!(session.group_name.is_none());
        assert_eq!(session.copies_printed, 0);
    }

    #[test]
    fn test_is_complete() {
        let mut session = Session::new();
        assert!(!session.is_complete());

        session.group_name = Some("Test Group".to_string());
        session.weapon = Some(1);
        session.land = Some(2);
        session.companion = Some(3);
        session.email = Some("test@example.com".to_string());
        session.photo_path = Some("/path/to/photo.png".to_string());
        session.story_text = Some("Test story".to_string());
        session.headline = Some("Test headline".to_string());

        assert!(session.is_complete());
    }

    #[test]
    fn test_generate_story() {
        let mut session = Session::new();
        session.weapon = Some(1);
        session.land = Some(2);
        session.companion = Some(3);

        session.generate_story();

        assert!(session.headline.is_some());
        assert!(session.story_text.is_some());
    }
}
