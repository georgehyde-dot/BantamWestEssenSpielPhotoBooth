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
    pub class: Option<i32>,
    pub choice: Option<i32>,
    pub land: Option<i32>,
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
            class: None,
            choice: None,
            land: None,
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
                id, group_name, created_at, class, choice, land,
                email, photo_path, copies_printed, story_text, headline
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
            "#,
        )
        .bind(&self.id)
        .bind(&self.group_name)
        .bind(&self.created_at)
        .bind(self.class)
        .bind(self.choice)
        .bind(self.land)
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
                class = ?3,
                choice = ?4,
                land = ?5,
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
        .bind(self.class)
        .bind(self.choice)
        .bind(self.land)
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
                id, group_name, created_at, class, choice, land,
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
            && self.class.is_some()
            && self.choice.is_some()
            && self.land.is_some()
            && self.email.is_some()
            && self.photo_path.is_some()
            && self.story_text.is_some()
            && self.headline.is_some()
    }

    pub fn generate_story(&mut self) {
        if let (Some(class_idx), Some(choice_idx), Some(land_idx)) =
            (self.class, self.choice, self.land)
        {
            // Define the names for classes, lands, and choices
            let classes = ["Gunslinger", "Merchant", "Thief", "Arsonist"];
            let lands = [
                "a broken wagon at a fork in the road",
                "a mine shaft entrance",
                "distant mountain swamplands",
                "a cabin by a stream",
            ];
            let choices = [
                "At high noon, face to face, pistol steady.",
                "By standing between the good folks and the bad ones.",
                "With my fist and a shot of whiskey.",
                "Through a shooting challenge — fair, no hard feelings.",
                "Won the town from the mayor in a single poker hand.",
                "Gave away half my fortune to the townsfolk.",
                "Sold snake oil to a snake.",
                "Controlled the entire gunpowder supply.",
                "Snatched the prized treasure from a crooked tycoon.",
                "Stole the Marshal’s keys and freed my friends in jail.",
                "Returned a stolen jewel to its rightful owner.",
                "Stole a wagonload of candy from a bunch of babies.",
                "The corrupt mayor’s mansion, lit up like judgment day.",
                "The saloon’s piano, mid-song, while I kept playing.",
                "A gun powder factory.",
                "The gambling hall—luck burned faster than the cards.",
            ];

            let class = classes
                .get(class_idx as usize)
                .unwrap_or(&"A hooded stranger");
            let choice = choices
                .get(choice_idx as usize)
                .unwrap_or(&"No Options Left");
            let land = lands
                .get(land_idx as usize)
                .unwrap_or(&"off into the sunset");

            // --- UPDATED STORIES ---
            // The stories now directly reflect the chosen class and action.
            let story = match (class_idx, choice_idx) {
                // Gunslinger Stories
                (0, 0) => format!("They said the only way to settle things was the old way. So there, by {}, the {} stood their ground. The legend of how they won? '{}'", land, class, choice),
                (0, 1) => format!("Trouble was brewing, but near {}, one figure stood tall. The {} earned their reputation '{}'", land, class, choice),
                (0, 2) => format!("Some problems can't be solved with words. Near {}, the {} proved that. They settled the dispute '{}'", land, class, choice),
                (0, 3) => format!("It wasn't about violence, it was about skill. At {}, the {} made a name for themself '{}'", land, class, choice),

                // Merchant Stories
                (1, 4) => format!("Power in the west isn't always won with a gun. Near {}, a crafty {} changed the whole territory. How? They '{}'", land, class, choice),
                (1, 5) => format!("A true {} knows that wealth is for more than just keeping. Near {}, they became a local hero when they '{}'", land, class, choice),
                (1, 6) => format!("Every {} needs a silver tongue. Out by {}, they pulled off their most legendary swindle when they '{}'", land, class, choice),
                (1, 7) => format!("The greatest {} doesn't just sell goods, they control the market. From their post at {}, they changed the game because they '{}'", land, class, choice),

                // Thief Stories
                (2, 8) => format!("Some called it crime, but the {} called it justice. Their greatest act of redistribution started at {}, where they '{}'", land, class, choice),
                (2, 9) => format!("Loyalty is worth more than gold. Near {}, the {} risked it all for their crew. They tell the tale of how they '{}'", land, class, choice),
                (2, 10) => format!("Even a {} with sticky fingers can have a heart of gold. By {}, they righted a terrible wrong when they '{}'", land, class, choice),
                (2, 11) => format!("Not every score is for riches. The {} once pulled a comical heist near {}, and all they did was '{}'", land, class, choice),

                // Arsonist Stories
                (3, 12) => format!("Sometimes, the only way to cleanse a town is with fire. From {}, the {} watched their handiwork: '{}'", land, class, choice),
                (3, 13) => format!("The {} was an artist, and their medium was chaos. Their masterpiece began near {} with '{}'", land, class, choice),
                (3, 14) => format!("A message needed to be sent, loud and clear. The {} sent it from {}, targeting '{}' The explosion was heard for miles.", land, class, choice),
                (3, 15) => format!("The house always wins, they say. Not when the {} is dealing. By {}, they ensured no one would ever cheat there again by igniting '{}'", land, class, choice),

                // Default fallback
                _ => format!("The story of the {} near {} is one for the ages, defined by a single, legendary choice: '{}'", class, land, choice),
            };

            // --- UPDATED HEADLINES ---
            // Headlines are now matched to the specific choice for better relevance.
            let headline = match choice_idx {
                0 => "High Noon Reckoning",
                1 => "The Town's Shield",
                2 => "Whiskey & Bruised Knuckles",
                3 => "The Sharpshooter's Challenge",
                4 => "The Mayor's Losing Hand",
                5 => "A Fortune for the Folk",
                6 => "The Serpent's Swindle",
                7 => "The Gunpowder Gambit",
                8 => "The Tycoon's Treasure",
                9 => "The Marshal's Keys",
                10 => "A Jewel for Justice",
                11 => "The Great Candy Caper",
                12 => "Mansion in Flames",
                13 => "A Fiery Tune",
                14 => "The Factory's Final Boom",
                15 => "Luck Runs Out",
                _ => "A Legend is Born",
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
        session.class = Some(1);
        session.choice = Some(2);
        session.land = Some(3);
        session.email = Some("test@example.com".to_string());
        session.photo_path = Some("/path/to/photo.png".to_string());
        session.story_text = Some("Test story".to_string());
        session.headline = Some("Test headline".to_string());

        assert!(session.is_complete());
    }

    #[test]
    fn test_generate_story() {
        let mut session = Session::new();
        session.class = Some(1);
        session.choice = Some(2);
        session.land = Some(3);

        session.generate_story();

        assert!(session.headline.is_some());
        assert!(session.story_text.is_some());
    }
}
