use crate::card::Card;
use crate::rng::GameRng;

/// Count the number of lands in a hand
fn count_lands(hand: &[Card]) -> usize {
    hand.iter()
        .filter(|c| matches!(c, Card::Land(_)))
        .count()
}

/// Score a hand based on playability (land count and mana curve)
/// Lower score is better
fn score_hand(hand: &[Card]) -> (i32, u32) {
    let land_count = count_lands(hand) as i32;
    
    // Ideal land count is 2-4 lands in a 7-card hand
    let land_penalty = if land_count < 2 {
        (2 - land_count) * 10  // Heavy penalty for too few lands
    } else if land_count > 4 {
        (land_count - 4) * 5   // Lighter penalty for too many lands
    } else {
        0  // No penalty for 2-4 lands
    };

    // Calculate mana curve (sum of mana values)
    let mana_curve: u32 = hand.iter().map(|c| c.mana_value()).sum();

    (land_penalty, mana_curve)
}

/// Select opening hand using BO1 hand smoother algorithm
/// Draws two 7-card hands and returns the better one
pub fn select_opening_hand(
    library: &mut Vec<Card>,
    rng: &mut GameRng,
) -> Vec<Card> {
    // Draw two hands of 7
    let hand1: Vec<Card> = library.drain(0..7).collect();
    let hand2: Vec<Card> = library.drain(0..7).collect();

    let lands1 = count_lands(&hand1);
    let lands2 = count_lands(&hand2);

    let (chosen_hand, rejected_hand) = if lands1 >= 2 && lands2 >= 2 {
        // Both hands have at least 2 lands, pick the one with fewer lands
        let score1 = score_hand(&hand1);
        let score2 = score_hand(&hand2);

        if score1 < score2 {
            (hand1, hand2)
        } else if score2 < score1 {
            (hand2, hand1)
        } else {
            // Same score, random pick
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
        // Put both hands back and shuffle
        library.extend(hand1);
        library.extend(hand2);
        rng.shuffle(library);
        return mulligan_hand(library, 6, rng);
    };

    // Put rejected hand back into library and shuffle
    library.extend(rejected_hand);
    rng.shuffle(library);

    chosen_hand
}

/// Mulligan to a smaller hand size
fn mulligan_hand(library: &mut Vec<Card>, hand_size: usize, rng: &mut GameRng) -> Vec<Card> {
    let hand: Vec<Card> = library.drain(0..hand_size).collect();

    let lands = count_lands(&hand);
    if lands < 2 && hand_size > 4 {
        // Still bad, mulligan again
        library.extend(hand);
        rng.shuffle(library);
        return mulligan_hand(library, hand_size - 1, rng);
    }

    hand
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
    fn test_score_hand() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        // Good hand: 3 lands, low curve
        let good_hand = vec![
            forest.clone(),
            forest.clone(),
            forest.clone(),
            terror.clone(),
            terror.clone(),
            terror.clone(),
            terror.clone(),
        ];

        let (penalty, _curve) = score_hand(&good_hand);
        assert_eq!(penalty, 0, "Good hand should have no penalty");
    }
}

