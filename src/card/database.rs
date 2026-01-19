use crate::card::types::Card;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CardDatabaseError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Card not found: {0}")]
    CardNotFound(String),
    #[error("Invalid card data: {0}")]
    InvalidCard(String),
}

/// Card database that loads cards from JSON
pub struct CardDatabase {
    cards: HashMap<String, Card>,
}

impl CardDatabase {
    /// Load cards from a JSON file
    pub fn from_file(path: &str) -> Result<Self, CardDatabaseError> {
        let content = std::fs::read_to_string(path)?;
        let cards_vec: Vec<Card> = serde_json::from_str(&content)?;

        let mut cards = HashMap::new();
        for card in cards_vec {
            let name = card.name().to_string();
            cards.insert(name, card);
        }

        Ok(CardDatabase { cards })
    }

    /// Get a card by name
    pub fn get_card(&self, name: &str) -> Result<Card, CardDatabaseError> {
        self.cards
            .get(name)
            .cloned()
            .ok_or_else(|| CardDatabaseError::CardNotFound(name.to_string()))
    }

    /// Get all card names
    pub fn card_names(&self) -> Vec<&str> {
        self.cards.keys().map(|s| s.as_str()).collect()
    }

    /// Get total number of cards
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }

    /// Validate that all referenced abilities exist
    pub fn validate(&self) -> Result<(), CardDatabaseError> {
        // For now, just check that we have cards loaded
        if self.cards.is_empty() {
            return Err(CardDatabaseError::InvalidCard(
                "No cards loaded".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_cards() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        assert!(db.card_count() > 0, "Should have loaded cards");
    }

    #[test]
    fn test_get_card() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let card = db.get_card("Forest").expect("Forest should exist");
        assert_eq!(card.name(), "Forest");
    }

    #[test]
    fn test_card_not_found() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let result = db.get_card("Nonexistent Card");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_cards_accessible() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let names = db.card_names();
        assert!(names.len() > 0, "Should have card names");

        // Verify we can get each card
        for name in names {
            let card = db.get_card(name).expect(&format!("Should get card: {}", name));
            assert_eq!(card.name(), name);
        }
    }
}

