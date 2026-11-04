use crate::card::Card;
use crate::rng::GameRng;

/// Count the number of lands in a hand
fn count_lands(hand: &[Card]) -> usize {
    hand.iter()
        .filter(|c| matches!(c, Card::Land(_)))
        .count()
}

/// Check if a card is a mill/surveil/graveyard enabler
fn is_mill_enabler(card: &Card) -> bool {
    let name = card.name();
    matches!(
        name,
        "Town Greeter"
            | "Overlord of the Balemurk"
            | "Kiora, the Rising Tide"
            | "Cache Grab"
            | "Dredger's Insight"
            | "Awaken the Honored Dead"
            | "Dark Matter Manipulator"
            | "Does Machines"
            | "Fear of Missing Out"
            | "Beseech the Mirror"
            | "Bitter Triumph"
            | "Wreckage Wickerfolk"
            | "Roiling Dragonstorm"
            | "Corroding Dragonstorm"
            | "Bellowing Crier"
            | "Yuriko, Hope from the Shadows"
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

/// Score how much we want to keep a card when we put cards on the bottom
/// after a London mulligan. `copies_before` is the number of cards with the
/// same name that already scored higher; `land_rank` is the rank of a land
/// among the lands in the hand (0 = best land).
fn keep_score(card: &Card, copies_before: usize, land_rank: usize) -> i32 {
    if let Card::Land(land) = card {
        let base = match land_rank {
            0 | 1 => 1000, // the first two lands are always kept
            2 => 500,      // the third land is very important
            3 => 150,
            _ => 10,
        };
        // Prefer lands that enter untapped and make more colors
        let quality = if land.enters_tapped { 0 } else { 5 } + land.colors.len() as i32;
        return base + quality;
    }
    let first = copies_before == 0;
    let name = card.name();
    let base = if name == "Superior Spider-Man" {
        if first { 600 } else { 100 }
    } else if name == "Beseech the Mirror" {
        if first { 450 } else { 120 }
    } else if is_mill_enabler(card) {
        if first { 400 } else { 300 }
    } else if name == "Bringer of the Last Gift" {
        if first { 350 } else { 50 }
    } else if is_playable_early_spell(card) {
        200
    } else {
        80
    };
    base - card.mana_value() as i32
}

/// London mulligan: from a 7-card hand, put `n` cards on the bottom.
/// Returns (cards to keep, cards for the bottom of the library).
fn choose_bottom(hand: Vec<Card>, n: usize) -> (Vec<Card>, Vec<Card>) {
    if n == 0 {
        return (hand, Vec::new());
    }
    // Rank lands by quality, so land_rank 0 is the best land
    let mut land_quality: Vec<(usize, i32)> = hand.iter().enumerate()
        .filter_map(|(i, c)| match c {
            Card::Land(l) => Some((i, (if l.enters_tapped { 0 } else { 5 }) + l.colors.len() as i32)),
            _ => None,
        })
        .collect();
    land_quality.sort_by(|a, b| b.1.cmp(&a.1));
    let land_rank = |i: usize| land_quality.iter().position(|(j, _)| *j == i).unwrap_or(0);

    // Score spells in order so that later copies of a name count as extras
    let mut order: Vec<usize> = (0..hand.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(keep_score(&hand[i], 0, land_rank(i))));
    let mut scores = vec![0i32; hand.len()];
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for &i in &order {
        let copies = seen.entry(hand[i].name()).or_insert(0);
        scores[i] = keep_score(&hand[i], *copies, land_rank(i));
        *copies += 1;
    }

    let mut idx: Vec<usize> = (0..hand.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(scores[i]));
    let bottom_set: std::collections::HashSet<usize> = idx.iter().rev().take(n).copied().collect();
    let mut keep = Vec::new();
    let mut bottom = Vec::new();
    for (i, c) in hand.into_iter().enumerate() {
        if bottom_set.contains(&i) { bottom.push(c) } else { keep.push(c) }
    }
    (keep, bottom)
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

/// Resolve the opening hand with Arena's Bo1 rules and the London mulligan.
/// Every 7-card draw uses Bo1 hand smoothing (`bo1_opening_hand`: of two
/// hands, keep the one whose land count is closest to the deck's land ratio).
/// After the m-th mulligan, put m cards from the new 7 on the bottom. The
/// keep / mulligan decision looks at the hand after the bottom step. A
/// 4-card hand is always kept.
pub fn resolve_mulligans(
    library: &mut Vec<Card>,
    rng: &mut GameRng,
    deck_land_count: usize,
    deck_size: usize,
) -> Vec<Card> {
    let mut mulligans = 0usize;
    loop {
        let seven = bo1_opening_hand(library, rng, deck_land_count, deck_size);
        let (keep, bottom) = choose_bottom(seven, mulligans);
        if mulligans >= 3 || !should_mulligan(&keep, mulligans as u32) {
            // The library's top is index 0, so the bottom is the end
            library.extend(bottom);
            return keep;
        }
        library.extend(keep);
        library.extend(bottom);
        rng.shuffle(library);
        mulligans += 1;
    }
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

        let hand = resolve_mulligans(&mut library, &mut rng, 60, 60);

        // Should have a hand of at least 4 cards (minimum after mulligans)
        assert!(hand.len() >= 4, "Hand should have at least 4 cards");
        assert!(hand.len() <= 7, "Hand should have at most 7 cards");
    }

    #[test]
    fn test_london_bottom_keeps_lands_and_spider_man() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let c = |n: &str| db.get_card(n).expect(n);
        // 5 lands + Spider-Man + Valgavoth; bottom 2
        let hand = vec![c("Swamp"), c("Watery Grave"), c("Island"), c("Undercity Sewers"),
            c("Pit of Offerings"), c("Superior Spider-Man"), c("Valgavoth, Terror Eater")];
        let (keep, bottom) = choose_bottom(hand, 2);
        assert_eq!(keep.len(), 5);
        assert_eq!(bottom.len(), 2);
        assert!(keep.iter().any(|c| c.name() == "Superior Spider-Man"));
        assert_eq!(count_lands(&keep), 4, "a 5-land hand bottoms one land and the worst spell");
        assert!(bottom.iter().any(|c| c.name() == "Valgavoth, Terror Eater"));
        // The bottomed land is one that enters tapped
        let bottom_land = bottom.iter().find(|c| matches!(c, Card::Land(_))).expect("a land");
        assert!(matches!(bottom_land.name(), "Undercity Sewers" | "Pit of Offerings"));
    }

    #[test]
    fn test_london_bottom_never_drops_below_two_lands() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let c = |n: &str| db.get_card(n).expect(n);
        let hand = vec![c("Swamp"), c("Island"), c("Superior Spider-Man"), c("Beseech the Mirror"),
            c("Does Machines"), c("Bringer of the Last Gift"), c("Bringer of the Last Gift")];
        let (keep, _) = choose_bottom(hand, 3);
        assert_eq!(keep.len(), 4);
        assert_eq!(count_lands(&keep), 2);
        assert!(keep.iter().any(|c| c.name() == "Superior Spider-Man"));
        assert!(keep.iter().any(|c| c.name() == "Beseech the Mirror"));
    }

    #[test]
    fn test_resolve_mulligans_keeps_card_count() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let deck = crate::simulation::deck::parse_deck_file("deck_latest.txt", &db).expect("deck");
        let lands = count_lands(&deck);
        for seed in 0..300u64 {
            let mut rng = crate::rng::GameRng::new(Some(seed));
            let mut library = deck.clone();
            rng.shuffle(&mut library);
            let hand = resolve_mulligans(&mut library, &mut rng, lands, deck.len());
            assert!(hand.len() >= 4 && hand.len() <= 7);
            assert_eq!(hand.len() + library.len(), 60, "no card is lost or duplicated");
        }
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

