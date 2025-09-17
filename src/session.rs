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

    pub fn generate_story(&mut self) {
        if let (Some(weapon_idx), Some(land_idx), Some(companion_idx)) =
            (self.weapon, self.land, self.companion)
        {
            // Define the updated names for weapons, lands, and companions
            let weapons = ["Cascan Recurve", "Foreigner", "Handtorch", "Peacemaker"];
            let lands = [
                "a broken wagon at a fork in the road",
                "a mine shaft entrance",
                "distant mountain swamplands",
                "a cabin by a stream",
            ];
            let companions = [
                "Kila Graham",
                "Sneaky Pete",
                "Acacia Goodwin",
                "Helena Bindi",
            ];

            let weapon = weapons
                .get(weapon_idx as usize)
                .unwrap_or(&"a mysterious weapon");
            let land = lands.get(land_idx as usize).unwrap_or(&"an unknown land");
            let companion = companions
                .get(companion_idx as usize)
                .unwrap_or(&"a lone traveler");

            let story = match (weapon_idx, land_idx, companion_idx) {
                // Cascan Recurve combinations (Bow)
                (0, 0, _) => format!("At the {}, they drew their {}, while {} watched the dark path ahead.", land, weapon, companion),
                (0, 1, _) => format!("They scanned the {}, {} held steady, as {} tended the nearby campfire.", land, weapon, companion),
                (0, 2, _) => format!("They moved silently through the {}, {} in hand, with {} pointing toward the mountains.", land, weapon, companion),
                (0, 3, _) => format!("By the {}, they practiced with their {}, arrows whistling past as {} looked on.", land, weapon, companion),

                // Foreigner combinations (Hatchet)
                (1, 0, _) => format!("Sizing up the {}, they gripped their worn {}, while {} gathered supplies.", land, weapon, companion),
                (1, 1, _) => format!("They chopped firewood with the {} outside the {}, a steady rhythm that comforted {}.", weapon, land, companion),
                (1, 2, _) => format!("Their {} cut a path through the dense {}, making the journey easier for {}.", weapon, land, companion),
                (1, 3, _) => format!("They rested near the {}, carving wood with their {} as {} watched peacefully.", land, weapon, companion),

                // Handtorch combinations (Torch)
                (2, 0, _) => format!("They held the {} high, its light dancing on the {} as {} peered into the darkness.", weapon, land, companion),
                (2, 1, _) => format!("With the {} held aloft, they stepped into the {}, {} following close behind.", weapon, land, companion),
                (2, 2, _) => format!("The {} cut through the gloom of the {}, a small beacon for them and {}.", weapon, land, companion),
                (2, 3, _) => format!("As night fell on the {}, they lit the {}, sharing its simple warmth with {}.", land, weapon, companion),

                // Peacemaker combinations (Revolver)
                (3, 0, _) => format!("At the {}, their hand rested on their {}, a silent promise to protect {}.", land, weapon, companion),
                (3, 1, _) => format!("Firelight glinted off their {} at the {}; {} stayed quiet, sensing trouble.", weapon, land, companion),
                (3, 2, _) => format!("In the lawless {}, the only authority was their {}, a fact {} knew well.", land, weapon, companion),
                (3, 3, _) => format!("As the sun rose over the {}, they cleaned their {}, a quiet ritual shared with {}.", land, weapon, companion),

                // Default fallback
                _ => format!("Armed with their {}, they ventured through the {} with their loyal companion, {}.", weapon, land, companion),
            };

            let headline = match (weapon_idx, companion_idx) {
                (0, 0) => "The Hunter and the Guide",
                (0, 1) => "An Arrow for an Outlaw",
                (0, 2) => "The Convict's Aim",
                (0, 3) => "The Cowgirl Archer",
                (1, 0) => "Wilderness Warriors",
                (1, 1) => "The Hatchet and the Thief",
                (1, 2) => "A Fugitive's Resolve",
                (1, 3) => "An Axe to Grind",
                (2, 0) => "Into the Dark",
                (2, 1) => "A Thief in the Night",
                (2, 2) => "Escaping the Shadows",
                (2, 3) => "The Guiding Light",
                (3, 0) => "The Survivor's Six-Shooter",
                (3, 1) => "An Unlikely Alliance",
                (3, 2) => "Justice for the Fugitive",
                (3, 3) => "The Saloon Standoff",
                _ => "A New Chapter",
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
