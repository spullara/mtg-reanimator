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
    let mut to_bottom: Vec<Card> = Vec::new();
    let mut to_top: Vec<Card> = Vec::new();

    // Look at top scry_count cards
    let scry_cards: Vec<Card> = library.drain(0..scry_count.min(library.len())).collect();

    for card in scry_cards {
        let name = card.name();

        // Always bottom Bringer/Terror (want in graveyard, not hand)
        if name == "Bringer of the Last Gift" || name == "Terror of the Peaks" {
            to_bottom.push(card);
        }
        // Bottom lands if we have enough in hand
        else if matches!(card, Card::Land(_)) && hand_lands >= 3 {
            to_bottom.push(card);
        }
        // Bottom expensive spells if we're missing lands
        else if card.mana_value() >= 4 && hand_lands < 2 {
            to_bottom.push(card);
        } else {
            to_top.push(card);
        }
    }

    // Reconstruct library: top cards, then rest, then bottom cards
    let mut new_library = to_top;
    new_library.extend(library.drain(0..));
    new_library.extend(to_bottom);

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

/// Bo1 opening hand smoothing algorithm.
///
/// Draws two opening hands of `hand_size` cards from the shuffled library,
/// then picks the hand whose land count is closest to the ideal ratio
/// `(deck_land_count / deck_size) * hand_size`. On ties, picks randomly.
/// The rejected hand is shuffled back into the library.
///
/// This matches MTG Arena's Best-of-1 hand smoothing algorithm as described at
/// <https://avenger.games/arena_hand_smoothing.html>.
pub fn bo1_opening_hand(
    library: &mut Vec<Card>,
    rng: &mut GameRng,
    deck_land_count: usize,
    deck_size: usize,
) -> Vec<Card> {
    let hand_size = 7;
    assert!(
        library.len() >= hand_size * 2,
        "Library must have at least {} cards to draw two hands of {}",
        hand_size * 2,
        hand_size,
    );

    // Draw two hands
    let hand1: Vec<Card> = library.drain(0..hand_size).collect();
    let hand2: Vec<Card> = library.drain(0..hand_size).collect();

    let lands1 = count_lands(&hand1);
    let lands2 = count_lands(&hand2);

    // Ideal land count for a hand of this size
    let ideal = (deck_land_count as f64 / deck_size as f64) * hand_size as f64;

    let dist1 = (lands1 as f64 - ideal).abs();
    let dist2 = (lands2 as f64 - ideal).abs();

    let (chosen, rejected) = if dist1 < dist2 {
        (hand1, hand2)
    } else if dist2 < dist1 {
        (hand2, hand1)
    } else {
        // Tie — pick randomly
        if rng.random() < 0.5 {
            (hand1, hand2)
        } else {
            (hand2, hand1)
        }
    };

    // Shuffle rejected hand back into library
    library.extend(rejected);
    rng.shuffle(library);

    chosen
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
            // Same land count, random pick (matches TypeScript behavior)
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
        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");
        assert!(is_mill_enabler(&town_greeter));

        let overlord = db.get_card("Overlord of the Balemurk").expect("Overlord should exist");
        assert!(is_mill_enabler(&overlord));

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
        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");

        // Hand with 1 land but has mill enabler - should keep
        let hand = vec![forest.clone(), town_greeter.clone(), forest.clone(), forest.clone(), forest.clone(), forest.clone(), forest.clone()];
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

    #[test]
    fn test_bo1_opening_hand_returns_seven_cards() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let mut rng = crate::rng::GameRng::new(Some(100));
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        // 24 lands, 36 spells — typical 60-card deck
        let mut library: Vec<Card> = Vec::new();
        for _ in 0..24 { library.push(forest.clone()); }
        for _ in 0..36 { library.push(terror.clone()); }
        rng.shuffle(&mut library);

        let hand = bo1_opening_hand(&mut library, &mut rng, 24, 60);
        assert_eq!(hand.len(), 7, "Bo1 hand should always be 7 cards");
        // 14 cards were drawn (two hands of 7), 7 returned → library should have 60-7 = 53
        assert_eq!(library.len(), 53, "Library should have 53 cards remaining");
    }

    #[test]
    fn test_bo1_opening_hand_prefers_closer_to_ideal() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        // Ideal for 24 lands in 60-card deck = 24/60 * 7 = 2.8
        // Construct a library so hand1 has 3 lands (dist 0.2) and hand2 has 0 lands (dist 2.8)
        // Hand1 = first 7: 3 lands + 4 spells
        // Hand2 = next 7: 0 lands + 7 spells
        let mut library: Vec<Card> = Vec::new();
        for _ in 0..3 { library.push(forest.clone()); }
        for _ in 0..4 { library.push(terror.clone()); }
        for _ in 0..7 { library.push(terror.clone()); }
        // Fill remaining to have enough for the library
        for _ in 0..14 { library.push(forest.clone()); }
        for _ in 0..25 { library.push(terror.clone()); }

        // Don't shuffle — we want deterministic hand composition
        let mut rng = crate::rng::GameRng::new(Some(42));
        let hand = bo1_opening_hand(&mut library, &mut rng, 24, 60);

        let lands = count_lands(&hand);
        // Hand1 (3 lands, dist=0.2) should beat hand2 (0 lands, dist=2.8)
        assert_eq!(lands, 3, "Should pick the hand with 3 lands (closer to ideal 2.8)");
    }

    #[test]
    fn test_bo1_opening_hand_picks_fewer_lands_when_both_above_ideal() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        // Ideal = 24/60 * 7 = 2.8
        // Hand1: 5 lands (dist = 2.2), hand2: 3 lands (dist = 0.2)
        let mut library: Vec<Card> = Vec::new();
        for _ in 0..5 { library.push(forest.clone()); }
        for _ in 0..2 { library.push(terror.clone()); }
        for _ in 0..3 { library.push(forest.clone()); }
        for _ in 0..4 { library.push(terror.clone()); }
        // Fill rest
        for _ in 0..16 { library.push(forest.clone()); }
        for _ in 0..30 { library.push(terror.clone()); }

        let mut rng = crate::rng::GameRng::new(Some(42));
        let hand = bo1_opening_hand(&mut library, &mut rng, 24, 60);

        let lands = count_lands(&hand);
        assert_eq!(lands, 3, "Should pick the hand with 3 lands (closer to ideal 2.8)");
    }

    #[test]
    fn test_bo1_opening_hand_tie_still_returns_valid_hand() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        // Ideal = 24/60 * 7 = 2.8
        // Hand1: 3 lands (dist = 0.2), hand2: 3 lands (dist = 0.2) → tie
        let mut library: Vec<Card> = Vec::new();
        for _ in 0..3 { library.push(forest.clone()); }
        for _ in 0..4 { library.push(terror.clone()); }
        for _ in 0..3 { library.push(forest.clone()); }
        for _ in 0..4 { library.push(terror.clone()); }
        for _ in 0..18 { library.push(forest.clone()); }
        for _ in 0..28 { library.push(terror.clone()); }

        let mut rng = crate::rng::GameRng::new(Some(42));
        let hand = bo1_opening_hand(&mut library, &mut rng, 24, 60);

        assert_eq!(hand.len(), 7);
        let lands = count_lands(&hand);
        assert_eq!(lands, 3, "Both hands have 3 lands, either is fine");
    }

    #[test]
    fn test_bo1_opening_hand_statistical_bias() {
        // Run many iterations and verify that average land count is
        // closer to ideal than a single random draw would give.
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let deck_lands = 24usize;
        let deck_size = 60usize;
        let ideal = (deck_lands as f64 / deck_size as f64) * 7.0; // 2.8

        let mut total_dist = 0.0;
        let iterations = 1000;
        for seed in 0..iterations {
            let mut library: Vec<Card> = Vec::new();
            for _ in 0..deck_lands { library.push(forest.clone()); }
            for _ in 0..(deck_size - deck_lands) { library.push(terror.clone()); }

            let mut rng = crate::rng::GameRng::new(Some(seed));
            rng.shuffle(&mut library);

            let hand = bo1_opening_hand(&mut library, &mut rng, deck_lands, deck_size);
            let lands = count_lands(&hand);
            total_dist += (lands as f64 - ideal).abs();
        }
        let avg_dist = total_dist / iterations as f64;
        // With smoothing, average distance to ideal should be noticeably < 1.0
        // A single random draw from hypergeometric gives avg distance ~1.05
        assert!(
            avg_dist < 1.0,
            "Bo1 smoothing should bring avg distance below 1.0, got {}",
            avg_dist,
        );
    }
}

