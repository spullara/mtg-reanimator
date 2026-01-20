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

    /// Get total number of cards
    pub fn card_count(&self) -> usize {
        self.cards.len()
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
}

