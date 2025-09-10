use chrono::Utc;
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
        if let (Some(weapon_idx), Some(land_idx), Some(companion_idx)) =
            (self.weapon, self.land, self.companion)
        {
            // Define the actual names based on the image files
            let weapons = ["sword", "hammer", "pistol", "fire spell"];
            let lands = [
                "misty hills",
                "abandoned house",
                "dusty road",
                "babbling stream",
            ];
            let companions = ["clever cat", "loyal dog", "wise duck", "noble horse"];

            // Get the selected items
            let weapon = weapons.get(weapon_idx as usize).unwrap_or(&"weapon");
            let land = lands.get(land_idx as usize).unwrap_or(&"land");
            let companion = companions
                .get(companion_idx as usize)
                .unwrap_or(&"companion");

            // Generate different story templates based on combinations
            let story = match (weapon_idx, land_idx, companion_idx) {
                // Sword combinations
                (0, 0, _) => format!("With {} drawn and their {} by their side, they climbed the {} seeking ancient treasure.", weapon, companion, land),
                (0, 1, _) => format!("Armed with a {} and accompanied by their {}, they explored the {} for hidden secrets.", weapon, companion, land),
                (0, 2, _) => format!("The {} gleamed in the sun as they traveled the {} with their faithful {}.", weapon, land, companion),
                (0, 3, _) => format!("By the {}, they unsheathed their {} while their {} stood guard.", land, weapon, companion),

                // Hammer combinations
                (1, 0, _) => format!("The mighty {} thundered as they conquered the {} alongside their brave {}.", weapon, land, companion),
                (1, 1, _) => format!("With {} in hand, they broke through the {}'s mysteries, their {} leading the way.", weapon, land, companion),
                (1, 2, _) => format!("Down the {}, the {} rang out with each step, their {} marching alongside.", land, weapon, companion),
                (1, 3, _) => format!("Near the {}, they forged ahead with their {}, the {} splashing playfully.", land, weapon, companion),

                // Pistol combinations
                (2, 0, _) => format!("The {} echoed across the {} as they stood ready with their vigilant {}.", weapon, land, companion),
                (2, 1, _) => format!("In the shadows of the {}, they gripped their {} while their {} watched for danger.", land, weapon, companion),
                (2, 2, _) => format!("The {} was dusty but their {} was clean, and their {} never left their side.", land, weapon, companion),
                (2, 3, _) => format!("By the peaceful {}, they holstered their {} to rest with their tired {}.", land, weapon, companion),

                // Fire spell combinations
                (3, 0, _) => format!("Magic crackled as they cast their {} atop the {}, their {} mesmerized by the flames.", weapon, land, companion),
                (3, 1, _) => format!("The {} illuminated the dark {}, while their {} cowered from the magical light.", weapon, land, companion),
                (3, 2, _) => format!("Sparks flew from their {} as they walked the {}, their {} keeping a safe distance.", weapon, land, companion),
                (3, 3, _) => format!("Steam rose where their {} met the {}, creating rainbows that delighted their {}.", weapon, land, companion),

                // Default fallback
                _ => format!("Armed with their {}, they ventured through the {} with their loyal {} companion.", weapon, land, companion),
            };

            // Generate headlines based on the combination
            let headline = match (weapon_idx, companion_idx) {
                (0, 0) => "The Feline Swordmaster",
                (0, 1) => "The Canine Knight",
                (0, 2) => "The Duck Defender",
                (0, 3) => "The Equine Warrior",
                (1, 0) => "The Hammer & The Cat",
                (1, 1) => "The Hound's Thunder",
                (1, 2) => "The Mighty Mallard",
                (1, 3) => "The Stallion's Strength",
                (2, 0) => "The Gunslinger's Cat",
                (2, 1) => "The Deputy Dog",
                (2, 2) => "The Quick-Draw Duck",
                (2, 3) => "The Horse Ranger",
                (3, 0) => "The Mystic Mouser",
                (3, 1) => "The Wizard's Hound",
                (3, 2) => "The Spell-Casting Duck",
                (3, 3) => "The Magical Mare",
                _ => "The Grand Adventure",
            };

            self.headline = Some(headline.to_string());
            self.story_text = Some(story);
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
