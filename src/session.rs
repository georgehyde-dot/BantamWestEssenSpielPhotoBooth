use chrono::Utc;
use rand::Rng;
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
    pub email: Option<String>,
    pub photo_path: Option<String>,
    pub copies_printed: i32,
    pub story_text: Option<String>,
    pub headline: Option<String>,
    pub mailing_list: i32,
}

impl Session {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            group_name: None,
            created_at: Utc::now().to_rfc3339(),
            class: None,
            choice: None,
            email: None,
            photo_path: None,
            copies_printed: 0,
            story_text: None,
            headline: None,
            mailing_list: 0,
        }
    }

    pub async fn save(&self, pool: &SqlitePool) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO session (
                id, group_name, created_at, class, choice,
                email, photo_path, copies_printed, story_text, headline, mailing_list
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
        .bind(&self.email)
        .bind(&self.photo_path)
        .bind(self.copies_printed)
        .bind(&self.story_text)
        .bind(&self.headline)
        .bind(&self.mailing_list)
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
                email = ?5,
                photo_path = ?6,
                copies_printed = ?7,
                story_text = ?8,
                headline = ?9,
                mailing_list = ?10
            WHERE id = ?1
            "#,
        )
        .bind(&self.id)
        .bind(&self.group_name)
        .bind(self.class)
        .bind(self.choice)
        .bind(&self.email)
        .bind(&self.photo_path)
        .bind(self.copies_printed)
        .bind(&self.story_text)
        .bind(&self.headline)
        .bind(&self.mailing_list)
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed(format!("Failed to update session: {}", e)))?;

        Ok(())
    }

    pub async fn load(id: &str, pool: &SqlitePool) -> AppResult<Option<Self>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, group_name, created_at, class, choice,
                email, photo_path, copies_printed, story_text, headline, mailing_list
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
            && self.email.is_some()
            && self.photo_path.is_some()
            && self.story_text.is_some()
            && self.headline.is_some()
    }

    pub fn generate_story(&mut self) {
        if let (Some(class_idx), Some(choice_idx)) = (self.class, self.choice) {
            let lands = [
                "a broken wagon at a fork in the road",
                "a mine shaft entrance",
                "distant mountain swamplands",
                "a cabin by a stream",
            ];

            // This 2D array holds 4 caption options for each of the 16 choices.
            let story_options: [[&str; 4]; 16] = [
                // 0: Gunslinger - High Noon
                [
                    "WANTED: DEAD OR ALIVE\nFor settling disputes the old-fashioned way.\nLast seen at high noon near {land}.\nAnswers only to their own code.",
                    "WANTED FOR DUELING\nThis gunslinger's draw is faster than a lightning strike.\nLeft a rival staring at the sun near {land}.\nDo not challenge.",
                    "REWARD OFFERED\nFor the duelist who settles all disputes at high noon.\nTheir legend was forged in the dust near {land}.\nApproach only to pay respects, or a price.",
                    "BE ADVISED\nThis individual solves all arguments with cold steel.\nAnother notch was added to their pistol near {land}.\nNegotiation is not an option."
                ],
                // 1: Gunslinger - Protector
                [
                    "WANTED FOR VIGILANTISM\nKnown to appoint themself judge, jury, and protector.\nLast seen defending townsfolk near {land}.\nConsiders the law a suggestion.",
                    "SOUGHT FOR QUESTIONING\nRegarding interference with outlaw business.\nThis do-gooder is a thorn in the side of \"progress\".\nLast seen near {land}.",
                    "NOTICE: THE TOWN'S GUARDIAN\nStands between the innocent and the wicked.\nLast seen making the roads safe near {land}.\n A hero to many, a target for some.",
                    "FOR HIRE: ONE GUN\nWill stand against any threat for the right price.\nProvided a service for the folk near {land}.\nTheir aim is true, their conscience debatable."
                ],
                // 2: Gunslinger - Brawler
                [
                    "APPROACH WITH CAUTION\nWanted for brawling and disorderly conduct.\nPrefers to let their fists do the talking.\nLast seen causing a ruckus near {land}.",
                    "WANTED: FOR TAVERN TERROR\nHas a taste for cheap whiskey and expensive fights.\nSettled a disagreement the hard way near {land}.\nKnown to have a mean right hook.",
                    "REWARD FOR INFORMATION\nLeading to the arrest of a known instigator.\nTheir temper is shorter than a watered-down drink.\nLast known disturbance was near {land}.",
                    "PUBLIC NUISANCE\nThis individual's arguments end in broken bottles.\nTheir knuckles are registered as lethal weapons.\nLast seen starting trouble near {land}."
                ],
                // 3: Gunslinger - Ruthless
                [
                    "WANTED: RUTHLESS KILLER\nFor crimes against humanity and common decency.\nNo one is safe from their bloodlust.\nLast seen leaving bodies near {land}.",
                    "BEWARE THE EXECUTIONER\nThis gunslinger believes in only one verdict: guilty.\nLeft no survivors to tell the tale near {land}.\nShows no mercy, expects none.",
                    "REWARD: DEAD OR ALIVE\nThis individual's justice is swift and final.\nTheir reputation for brutality was earned near {land}.\nInnocence is not a concept they recognize.",
                    "SOUGHT: FOR MASS MURDER\nWanted for indiscriminate killing.\nLeaves behind only silence and sorrow.\nLast seen dispensing death near {land}."
                ],
                // 4: Merchant - Poker
                [
                    "WANTED FOR CRIMES OF CUNNING\nThis smooth talker won a town charter in a poker game.\nAll deals should be considered suspect.\nLast known location: {land}.",
                    "NOTICE: CHANGE OF OWNERSHIP\nThe town charter was lost in a game of cards.\nThe new proprietor is a known gambler from {land}.\nAll debts are now due to them.",
                    "SOUGHT FOR QUESTIONING\nRegarding a suspicious hand of five aces.\nThe former mayor is demanding a recount.\nThe incident occurred near {land}.",
                    "REWARD: FOR THE CARD SHARK\nWanted for winning more than just the pot.\nThis high-stakes player now runs the town.\nLast seen shuffling a deck near {land}."
                ],
                // 5: Merchant - Charity
                [
                    "SOUGHT FOR QUESTIONING\nRegarding suspicious and disruptive charity.\nKnown for upending the local economy.\nLast seen distributing their fortune near {land}.",
                    "WANTED: ECONOMIC ANARCHIST\nThis so-called 'benefactor' is devaluing local currency.\nTheir generosity is a threat to the natural order.\nLast seen making it rain near {land}.",
                    "BEWARE FALSE PROPHETS\nThis merchant gives with one hand and takes with... well, we're not sure yet.\nTheir motives are unknown.\nLast seen near {land}.",
                    "NOTICE OF UNCLAIMED WEALTH\nThis individual is handing out gold like it's candy.\nSuch actions have consequences.\nThe spectacle was witnessed near {land}."
                ],
                // 6: Merchant - Snake Oil
                [
                    "WANTED FOR FRAUD\nSo slick they could sell a mirage to a man dying of thirst.\nPeddles elixirs of questionable origin.\nLast spotted near {land}.",
                    "BEWARE THE SILVER TONGUE\nThis charlatan's promises are as empty as their bottles.\nPulled off their greatest swindle near {land}.\nWill sell you the rope to hang yourself with.",
                    "REWARD FOR APPREHENSION\nOf the most notorious con artist in the territories.\nTheir 'miracle cure' is 90% ditch water.\nLast seen fleeing {land}.",
                    "PUBLIC WARNING\nDo not buy *anything* from this individual.\nTheir salesmanship is a registered hazard.\nLast seen charming the locals near {land}."
                ],
                // 7: Merchant - Gunpowder
                [
                    "WANTED: MONOPOLIST\nFor cornering the market on all things that go 'BOOM'.\nThis merchant's ambition is a threat to public safety.\nOperates out of {land}.",
                    "DANGEROUS INDIVIDUAL\nControls the flow of gunpowder and lead.\nEffectively holds the entire territory hostage.\nTheir main stockpile is near {land}.",
                    "REWARD FOR INFORMATION\nOn the merchant who holds the keys to the armory.\nHe who controls the powder, controls the war.\nHQ rumored to be near {land}.",
                    "SOUGHT FOR PRICE GOUGING\nThis merchant has made peace an expensive luxury.\nSells bullets at a premium.\nLast seen counting their money near {land}."
                ],
                // 8: Thief - Tycoon
                [
                    "WANTED FOR 'REDISTRIBUTION'\nA folk hero to some, a menace to the rich.\nLiberates treasure from the undeserving.\nLast known score occurred near {land}.",
                    "REWARD: FOR THE PEOPLE'S THIEF\nStole from the rich to give to... well, themself mostly.\nBut the tycoon deserved it.\nThe heist took place near {land}.",
                    "SOUGHT FOR GRAND LARCENY\nTargeted the holdings of a corrupt railroad baron.\nThe stolen goods have not been recovered.\nLast seen celebrating near {land}.",
                    "NOTICE: JUSTICE SERVED\nThe so-called 'Tycoon's Treasure' is now in new hands.\nThe perpetrator is a local legend.\nThe act of defiance happened near {land}."
                ],
                // 9: Thief - Jailbreak
                [
                    "SOUGHT FOR AIDING FUGITIVES\nValues loyalty to their crew above the law.\nOrchestrated a brazen jailbreak near {land}.\nConsidered armed and resourceful.",
                    "WANTED: FOR OBSTRUCTION\nThis thief stole the Marshal's keys and his dignity.\nResponsible for releasing known criminals.\nLast seen with their gang near {land}.",
                    "REWARD FOR CAPTURE\nOf the mastermind behind the {land} jailbreak.\nMade a mockery of the local law enforcement.\nLoyal, cunning, and dangerous.",
                    "BE ADVISED\nA band of outlaws is on the loose.\nThanks to the efforts of one very skilled thief.\nThe escape originated near {land}."
                ],
                // 10: Thief - Jewel Return
                [
                    "WANTED... FOR RETURNING STOLEN GOODS?\nAn unpredictable agent of justice.\nTheir strange reversal of fortune took place near {land}.\nMotive: Unknown.",
                    "SOUGHT FOR QUESTIONING\nRegarding a case of reverse-robbery.\nThis thief has a peculiar moral code.\nThe incident baffled deputies near {land}.",
                    "BEWARE THE GHOST THIEF\nSteals from the guilty, returns to the innocent.\nTheir latest act of strange justice occurred near {land}.\nOperates outside of any known law.",
                    "NOTICE: A CONSCIENCE\nEven a thief can right a wrong.\nA stolen jewel was mysteriously returned near {land}.\nThis individual is an enigma."
                ],
                // 11: Thief - Candy
                [
                    "WANTED FOR PETTY CRIMES\nThis villain's depravity knows no bounds.\nTheir last heist involved candy and babies.\nApprehend for the sake of decency near {land}.",
                    "SOUGHT FOR QUESTIONING\nRegarding a sudden, tragic shortage of lollipops.\nThe suspect was last seen fleeing {land}.\nConsidered sticky-fingered and shameless.",
                    "CRIME OF THE CENTURY\nWanted for a brazen daylight candy robbery.\nThe victims were unarmed and mostly toothless.\nLast seen with a bulging sack near {land}.",
                    "NOTICE: A VILLAIN AMONG US\nThis fiend stooped so low as to steal from a child.\nThe great candy caper of {land} will not be forgotten.\nThere is no honor among this thief."
                ],
                // 12: Arsonist - Mansion
                [
                    "WANTED FOR ARSON\nDispenses fiery justice against corrupt officials.\nThe mayor's mansion near {land} was their last target.\nBelieved to be armed with kerosene.",
                    "REWARD FOR INFORMATION\nOn the firebrand who lit up the mayor's night.\nSent a very clear, very warm message to the establishment.\nThe blaze was started near {land}.",
                    "SOUGHT: POLITICAL PYRO\nUses flames to make their political statements.\nThe target was a symbol of corruption.\nLast seen watching the glow from {land}.",
                    "NOTICE: A CLEANSING FIRE\nThe mayor's ill-gotten gains went up in smoke.\nThe people's justice was delivered by matchstick.\nThe act took place near {land}."
                ],
                // 13: Arsonist - Piano
                [
                    "WANTED: PYROMANIAC\nAn artist whose medium is chaos and flame.\nLast seen turning a saloon piano into a bonfire.\nSpotted admiring their work near {land}.",
                    "SOUGHT FOR VANDALISM\nThis fiend gave a beloved piano a fiery send-off.\nThe music died in a blaze of glory near {land}.\nMotive appears to be pure, chaotic joy.",
                    "BEWARE THE FIREBUG\nFinds beauty in the blaze, and music in the crackle.\nTheir latest masterpiece was a piano near {land}.\nDo not leave flammable objects unattended.",
                    "REWARD: FOR THE SILENCER\nWanted for interrupting a perfectly good tune with fire.\nThe saloon regulars are not pleased.\nThe incident occurred near {land}."
                ],
                // 14: Arsonist - Factory
                [
                    "WANTED: GANG WARFARE\nThis pyromaniac escalated a feud to devastating levels.\nBurned a rival gang's hideout to the ground near {land}.\nConsidered extremely dangerous.",
                    "SOUGHT FOR MASS ARSON\nSettled old scores with fire and vengeance.\nLeft nothing but ashes of their enemies near {land}.\nThis individual takes no prisoners.",
                    "REWARD FOR CAPTURE\nOf the firebrand who eliminated an entire gang.\nTheir rivals' screams were heard throughout {land}.\nJustice or murder? The jury's still out.",
                    "BEWARE: GANG ELIMINATOR\nThis arsonist doesn't believe in second chances.\nTurned a turf war into a funeral pyre near {land}.\nTheir definition of 'victory' is total annihilation."
                ],
                // 15: Arsonist - Christmas Tree
                [
                    "WANTED FOR HOLIDAY HOOLIGANISM\nThis yuletide troublemaker lit up the season a bit too literally.\nTurned the town Christmas tree into the world's largest candle near {land}.\nSuspect may be a Grinch in disguise.",
                    "NOTICE: CHRISTMAS CANCELLED\nDue to one individual's overzealous interpretation of 'holiday lights'.\nThe town tree became a festive inferno near {land}.\nSanta has been notified and is NOT pleased.",
                    "SOUGHT: THE HOLIDAY ARSONIST\nRuined Christmas faster than finding coal in your stocking.\nWitnesses report cackling and possible eggnog involvement near {land}.\nMay have been singing carols while fleeing.",
                    "REWARD FOR THE SCROOGE\nWho confused 'deck the halls' with 'burn them all'.\nThe great Christmas tree disaster of {land} will go down in infamy.\nChildren are crying. The mayor is crying. Even the ornaments are crying."
                ],
            ];

            let land_idx = (class_idx + choice_idx) % lands.len() as i32;
            let land = lands
                .get(land_idx as usize)
                .unwrap_or(&"the empty wilderness");

            // --- RANDOM STORY SELECTION ---
            let mut rng = rand::thread_rng();
            let random_caption_idx = rng.gen_range(0..4);

            let captions_for_choice = story_options
                .get(choice_idx as usize)
                .unwrap_or(&[
                    "WANTED: FOR REASONS UNKNOWN\nThis mysterious figure was last seen near {land}.\nTheir motives are unclear.\nApproach with extreme caution.",
                    "SOUGHT: THE ENIGMA\nA shadow that passed through {land}.\nTheir purpose is a mystery, their methods unpredictable.\nReport any strange occurrences.",
                    "REWARD: FOR IDENTIFICATION\nOf a person of interest spotted near {land}.\nTheir story is unwritten, their legend just begun.\nDo not approach.",
                    "BE ADVISED\nAn unknown agent is operating in the area.\nTheir last known position was {land}.\nAssume nothing. Question everything."
                ]);

            let chosen_caption_template = captions_for_choice[random_caption_idx];
            let story = chosen_caption_template.replace("{land}", land);

            // Headlines remain the same for each choice
            let headline = match choice_idx {
                0 => "High Noon Reckoning",
                1 => "The Town's Shield",
                2 => "Whiskey & Bruised Knuckles",
                3 => "No Mercy, No Innocents",
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
                14 => "Ashes for my Enemies",
                15 => "Christmas Inferno",
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

        session.generate_story();

        assert!(session.headline.is_some());
        assert!(session.story_text.is_some());
    }
}
