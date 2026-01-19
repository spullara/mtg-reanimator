use crate::card::{Card, CardDatabase, CardDatabaseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeckError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid deck format at line {line}: {reason}")]
    InvalidFormat { line: usize, reason: String },
    #[error("Card database error: {0}")]
    DatabaseError(#[from] CardDatabaseError),
}

/// Parse a deck file and return expanded list of cards
/// Format: "4 Card Name" per line, supports comments with # or //
pub fn parse_deck_file(
    path: &str,
    database: &CardDatabase,
) -> Result<Vec<Card>, DeckError> {
    let content = std::fs::read_to_string(path)?;
    let mut deck = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        // Parse "N Card Name" format
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(DeckError::InvalidFormat {
                line: line_num + 1,
                reason: "Expected format: 'COUNT CARD_NAME'".to_string(),
            });
        }

        let count_str = parts[0];
        let card_name = parts[1].trim();

        let count: usize = count_str.parse().map_err(|_| DeckError::InvalidFormat {
            line: line_num + 1,
            reason: format!("'{}' is not a valid number", count_str),
        })?;

        // Get card from database
        let card = database.get_card(card_name)?;

        // Add card 'count' times
        for _ in 0..count {
            deck.push(card.clone());
        }
    }

    Ok(deck)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deck_file() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");
        
        // deck.txt should have 60 cards total
        assert_eq!(deck.len(), 60, "Deck should have 60 cards");
    }

    #[test]
    fn test_deck_expansion() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");
        
        // Count Forest cards (should be 2)
        let forest_count = deck.iter().filter(|c| c.name() == "Forest").count();
        assert_eq!(forest_count, 2, "Should have 2 Forest cards");
    }

    #[test]
    fn test_invalid_card_name() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let result = parse_deck_file("deck.txt", &db);
        
        // This should succeed since deck.txt has valid cards
        assert!(result.is_ok());
    }
}

