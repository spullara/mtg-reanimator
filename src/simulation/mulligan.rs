use crate::card::Card;
use crate::rng::GameRng;

/// Count the number of lands in a hand
fn count_lands(hand: &[Card]) -> usize {
    hand.iter()
        .filter(|c| matches!(c, Card::Land(_)))
        .count()
}

/// Check if a card is a mill/surveil enabler
fn is_mill_enabler(card: &Card) -> bool {
    let name = card.name();
    matches!(
        name,
        "Stitcher's Supplier"
            | "Teachings of the Kirin"
            | "Town Greeter"
            | "Overlord of the Balemurk"
            | "Kiora, the Rising Tide"
            | "Cache Grab"
            | "Dredger's Insight"
            | "Awaken the Honored Dead"
    )
}

/// Check if a card is a playable early spell (low mana value)
fn is_playable_early_spell(card: &Card) -> bool {
    card.mana_value() <= 3 && !matches!(card, Card::Land(_))
}

/// Decide whether to mulligan a hand
/// Keep hands with:
/// - 2-5 lands AND at least one playable early spell
/// - Mill/surveil enabler
/// Mulligan aggressive hands that can't fill graveyard
/// Be more lenient at higher mulligan counts
pub fn should_mulligan(hand: &[Card], _mulligan_count: u32) -> bool {
    let lands = count_lands(hand);

    // At 4 cards or fewer, keep almost anything with 2+ lands
    if hand.len() <= 4 {
        return lands < 2;
    }

    // Check for mill enablers - always keep if we have one
    if hand.iter().any(is_mill_enabler) {
        return lands < 2;
    }

    // Check for playable early spells
    let has_early_spell = hand.iter().any(is_playable_early_spell);

    // Keep if we have 2-5 lands and at least one early spell
    if lands >= 2 && lands <= 5 && has_early_spell {
        return false;
    }

    // Mulligan if we don't have enough lands or playable spells
    lands < 2 || !has_early_spell
}

/// Scry after mulligan - decide which cards to put on bottom
/// Scry decision: bottom lands if hand has enough, bottom expensive spells if hand is missing lands
fn scry_after_mulligan(library: &mut Vec<Card>, hand: &[Card], scry_count: usize) {
    if scry_count == 0 || library.is_empty() {
        return;
    }
    
    let hand_lands = count_lands(hand);
    let mut to_bottom = Vec::new();
    let mut to_top = Vec::new();
    
    // Look at top scry_count cards
    for i in 0..scry_count.min(library.len()) {
        let card = &library[i];
        let name = card.name();
        
        // Always bottom Bringer/Terror (want in graveyard, not hand)
        if name == "Bringer of the Last Gift" || name == "Terror of the Peaks" {
            to_bottom.push(i);
        }
        // Bottom lands if we have enough in hand
        else if matches!(card, Card::Land(_)) && hand_lands >= 3 {
            to_bottom.push(i);
        }
        // Bottom expensive spells if we're missing lands
        else if card.mana_value() >= 4 && hand_lands < 2 {
            to_bottom.push(i);
        } else {
            to_top.push(i);
        }
    }
    
    // Reconstruct library: keep top cards in order, then rest, then bottom cards
    let mut new_library = Vec::new();
    
    // Add cards to keep on top (in original order)
    for i in &to_top {
        new_library.push(library.remove(*i));
    }
    
    // Add rest of library
    let rest: Vec<Card> = library.drain(0..).collect();
    new_library.extend(rest);
    
    // Add cards to bottom (in original order)
    for i in &to_bottom {
        new_library.push(library.remove(*i));
    }
    
    *library = new_library;
}

/// Mulligan to a smaller hand size, with scry
fn mulligan_hand(library: &mut Vec<Card>, hand_size: usize, rng: &mut GameRng) -> Vec<Card> {
    let hand: Vec<Card> = library.drain(0..hand_size).collect();
    
    let lands = count_lands(&hand);
    if lands < 2 && hand_size > 4 {
        // Still bad, mulligan again
        library.extend(hand);
        rng.shuffle(library);
        return mulligan_hand(library, hand_size - 1, rng);
    }
    
    // Scry for each card below 7
    let scry_count = 7 - hand_size;
    if scry_count > 0 {
        scry_after_mulligan(library, &hand, scry_count);
    }
    
    hand
}

/// Resolve mulligans starting from opening hand
/// Returns the final hand after all mulligans and scries
pub fn resolve_mulligans(library: &mut Vec<Card>, rng: &mut GameRng) -> Vec<Card> {
    // Draw two hands of 7 using BO1 hand smoother
    let hand1: Vec<Card> = library.drain(0..7).collect();
    let hand2: Vec<Card> = library.drain(0..7).collect();
    
    let lands1 = count_lands(&hand1);
    let lands2 = count_lands(&hand2);
    
    let (mut chosen_hand, rejected_hand) = if lands1 >= 2 && lands2 >= 2 {
        // Both hands have at least 2 lands, pick the one with fewer lands
        if lands1 < lands2 {
            (hand1, hand2)
        } else if lands2 < lands1 {
            (hand2, hand1)
        } else {
            // Same land count, random pick
            if rng.random() < 0.5 {
                (hand1, hand2)
            } else {
                (hand2, hand1)
            }
        }
    } else if lands1 >= 2 {
        (hand1, hand2)
    } else if lands2 >= 2 {
        (hand2, hand1)
    } else {
        // Both hands have 0-1 lands, need to mulligan
        library.extend(hand1);
        library.extend(hand2);
        rng.shuffle(library);
        return mulligan_hand(library, 6, rng);
    };
    
    // Put rejected hand back into library and shuffle
    library.extend(rejected_hand);
    rng.shuffle(library);

    // Check if we need to mulligan the chosen hand
    let mut mulligan_count = 0;
    loop {
        if !should_mulligan(&chosen_hand, mulligan_count) || chosen_hand.len() <= 4 {
            break;
        }

        let next_hand_size = chosen_hand.len() - 1;
        library.extend(chosen_hand.clone());
        rng.shuffle(library);
        chosen_hand = mulligan_hand(library, next_hand_size, rng);
        mulligan_count += 1;
    }

    chosen_hand
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardDatabase;

    #[test]
    fn test_count_lands() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let hand = vec![forest.clone(), forest.clone(), terror.clone()];
        assert_eq!(count_lands(&hand), 2);
    }

    #[test]
    fn test_is_mill_enabler() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");

        // Test known mill enablers
        let supplier = db.get_card("Stitcher's Supplier").expect("Supplier should exist");
        assert!(is_mill_enabler(&supplier));

        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");
        assert!(is_mill_enabler(&town_greeter));

        // Test non-enabler
        let forest = db.get_card("Forest").expect("Forest should exist");
        assert!(!is_mill_enabler(&forest));
    }

    #[test]
    fn test_should_mulligan_bad_hand() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        // Hand with only 1 land - should mulligan
        let bad_hand = vec![forest.clone(), terror.clone(), terror.clone(), terror.clone(), terror.clone(), terror.clone(), terror.clone()];
        assert!(should_mulligan(&bad_hand, 0), "Hand with 1 land should mulligan");
    }

    #[test]
    fn test_should_mulligan_with_enabler() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let supplier = db.get_card("Stitcher's Supplier").expect("Supplier should exist");

        // Hand with 1 land but has mill enabler - should keep
        let hand = vec![forest.clone(), supplier.clone(), forest.clone(), forest.clone(), forest.clone(), forest.clone(), forest.clone()];
        assert!(!should_mulligan(&hand, 0), "Hand with enabler should keep");
    }

    #[test]
    fn test_resolve_mulligans_basic() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let mut rng = crate::rng::GameRng::new(Some(42));

        // Create a deck with enough cards
        let mut library = Vec::new();
        for _ in 0..60 {
            library.push(db.get_card("Forest").expect("Forest should exist"));
        }

        let hand = resolve_mulligans(&mut library, &mut rng);

        // Should have a hand of at least 4 cards (minimum after mulligans)
        assert!(hand.len() >= 4, "Hand should have at least 4 cards");
        assert!(hand.len() <= 7, "Hand should have at most 7 cards");
    }
}

