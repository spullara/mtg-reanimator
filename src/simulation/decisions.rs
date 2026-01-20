use crate::card::{Card, CardDatabase, CardType, LandCard, LandSubtype};
use crate::game::state::GameState;

/// Decision engine for MTG Reanimator AI
pub struct DecisionEngine;

impl DecisionEngine {
    /// Decide whether to mulligan a hand
    /// Keep if: 2+ lands AND (1+ mill enabler OR 1+ playable spell)
    pub fn should_mulligan(hand: &[Card], _mulligan_count: u32) -> bool {
        let lands = Self::count_lands(hand);

        // At 4 cards or fewer, keep almost anything with 2+ lands
        if hand.len() <= 4 {
            return lands < 2;
        }

        // Check for mill enablers - always keep if we have one
        if hand.iter().any(Self::is_mill_enabler) {
            return lands < 2;
        }

        // Check for playable early spells
        let has_early_spell = hand.iter().any(Self::is_playable_early_spell);

        // Keep if we have 2-5 lands and at least one early spell
        if lands >= 2 && lands <= 5 && has_early_spell {
            return false;
        }

        // Mulligan if we don't have enough lands or playable spells
        lands < 2 || !has_early_spell
    }

    /// Choose which card to play from hand - matches TypeScript's sophisticated logic
    /// Priority:
    /// 1. Spider-Man if combo is lethal
    /// 2. Kiora if Bringer/Terror in hand (to discard them)
    /// 3. Mill spells (Cache Grab, Dredger's Insight, Town Greeter, Overlord)
    /// 4. Awaken the Honored Dead (saga that mills)
    /// 5. Other spells by mana cost
    pub fn choose_card_to_play(
        hand: &[Card],
        state: &GameState,
        _db: &CardDatabase,
    ) -> Option<usize> {
        // Filter castable spells
        let mut castable: Vec<(usize, &Card)> = hand
            .iter()
            .enumerate()
            .filter(|(_, card)| {
                if matches!(card, Card::Land(_)) {
                    return false;
                }
                if !Self::can_cast(card, &state.mana_pool) {
                    return false;
                }

                // Only cast Spider-Man if combo would be lethal
                if card.name() == "Superior Spider-Man" {
                    let has_bringer_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
                    if !has_bringer_in_gy {
                        return false;
                    }
                    // Check if combo is lethal (simplified: if we have enough creatures)
                    let creature_count = state.battlefield.permanents().iter()
                        .filter(|p| matches!(p.card, Card::Creature(_)))
                        .count();
                    if creature_count < 2 {
                        return false; // Not enough creatures for lethal
                    }
                }

                true
            })
            .collect();

        if castable.is_empty() {
            return None;
        }

        // Check game state for priorities
        let has_bringer_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
        let has_bringer_in_hand = hand.iter().any(|c| c.name() == "Bringer of the Last Gift");
        let has_terror_in_hand = hand.iter().any(|c| c.name() == "Terror of the Peaks");
        let has_spider_in_hand = hand.iter().any(|c| c.name() == "Superior Spider-Man");
        let combo_is_lethal = has_bringer_in_gy && has_spider_in_hand && state.opponent_life <= 20; // Simplified check

        // Sort by priority
        castable.sort_by(|a, b| {
            let a_name = a.1.name();
            let b_name = b.1.name();

            // Priority 1: Spider-Man if combo is lethal
            if combo_is_lethal {
                if a_name == "Superior Spider-Man" {
                    return std::cmp::Ordering::Less;
                }
                if b_name == "Superior Spider-Man" {
                    return std::cmp::Ordering::Greater;
                }
            }

            // Priority 2: Kiora if Bringer or Terror in hand
            if has_bringer_in_hand || has_terror_in_hand {
                if a_name == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Less;
                }
                if b_name == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Greater;
                }
            }

            // Priority 3: Mill spells
            let mill_spells = ["Cache Grab", "Dredger's Insight", "Town Greeter", "Overlord of the Balemurk"];
            let a_is_mill = mill_spells.contains(&a_name);
            let b_is_mill = mill_spells.contains(&b_name);
            if a_is_mill && !b_is_mill {
                return std::cmp::Ordering::Less;
            }
            if b_is_mill && !a_is_mill {
                return std::cmp::Ordering::Greater;
            }

            // Priority 4: Awaken the Honored Dead
            if a_name == "Awaken the Honored Dead" && !b_is_mill {
                return std::cmp::Ordering::Less;
            }
            if b_name == "Awaken the Honored Dead" && !a_is_mill {
                return std::cmp::Ordering::Greater;
            }

            // Priority 5: Cheaper spells
            a.1.mana_value().cmp(&b.1.mana_value())
        });

        castable.first().map(|(idx, _)| *idx)
    }

    /// Choose which land to play - matches TypeScript's sophisticated logic
    /// Priority 0: Lands that enable casting something this turn
    /// Priority 1: Lands that provide missing colors (if neither enables casting)
    /// Priority 2: Surveil lands for value (if neither enables casting)
    /// Priority 3: Tapped lands (save untapped for later)
    pub fn choose_land_to_play(hand: &[Card], state: &GameState) -> Option<usize> {
        let lands: Vec<(usize, &Card)> = hand
            .iter()
            .enumerate()
            .filter_map(|(idx, card)| {
                if matches!(card, Card::Land(_)) {
                    Some((idx, card))
                } else {
                    None
                }
            })
            .collect();

        if lands.is_empty() {
            return None;
        }

        // Calculate available mana and colors
        let mut mana_available = 0;
        let mut colors_available = std::collections::HashSet::new();
        for perm in state.battlefield.permanents() {
            if let Card::Land(land) = &perm.card {
                if !perm.tapped {
                    mana_available += 1;
                    for color in &land.colors {
                        colors_available.insert(*color);
                    }
                }
            }
        }

        // Calculate mana after playing a land (one more untapped land)
        let mana_after_land_drop = mana_available + 1;

        // Get spells in hand
        let spells_in_hand: Vec<&Card> = hand
            .iter()
            .filter(|c| !matches!(c, Card::Land(_)))
            .collect();

        // Calculate missing colors
        let mut missing_colors = std::collections::HashSet::new();
        for spell in &spells_in_hand {
            match spell {
                Card::Creature(c) => {
                    if c.base.mana_cost.white > 0 && !colors_available.contains(&crate::card::ManaColor::White) {
                        missing_colors.insert(crate::card::ManaColor::White);
                    }
                    if c.base.mana_cost.blue > 0 && !colors_available.contains(&crate::card::ManaColor::Blue) {
                        missing_colors.insert(crate::card::ManaColor::Blue);
                    }
                    if c.base.mana_cost.black > 0 && !colors_available.contains(&crate::card::ManaColor::Black) {
                        missing_colors.insert(crate::card::ManaColor::Black);
                    }
                    if c.base.mana_cost.red > 0 && !colors_available.contains(&crate::card::ManaColor::Red) {
                        missing_colors.insert(crate::card::ManaColor::Red);
                    }
                    if c.base.mana_cost.green > 0 && !colors_available.contains(&crate::card::ManaColor::Green) {
                        missing_colors.insert(crate::card::ManaColor::Green);
                    }
                }
                Card::Enchantment(e) => {
                    if e.base.mana_cost.white > 0 && !colors_available.contains(&crate::card::ManaColor::White) {
                        missing_colors.insert(crate::card::ManaColor::White);
                    }
                    if e.base.mana_cost.blue > 0 && !colors_available.contains(&crate::card::ManaColor::Blue) {
                        missing_colors.insert(crate::card::ManaColor::Blue);
                    }
                    if e.base.mana_cost.black > 0 && !colors_available.contains(&crate::card::ManaColor::Black) {
                        missing_colors.insert(crate::card::ManaColor::Black);
                    }
                    if e.base.mana_cost.red > 0 && !colors_available.contains(&crate::card::ManaColor::Red) {
                        missing_colors.insert(crate::card::ManaColor::Red);
                    }
                    if e.base.mana_cost.green > 0 && !colors_available.contains(&crate::card::ManaColor::Green) {
                        missing_colors.insert(crate::card::ManaColor::Green);
                    }
                }
                Card::Sorcery(s) => {
                    if s.base.mana_cost.white > 0 && !colors_available.contains(&crate::card::ManaColor::White) {
                        missing_colors.insert(crate::card::ManaColor::White);
                    }
                    if s.base.mana_cost.blue > 0 && !colors_available.contains(&crate::card::ManaColor::Blue) {
                        missing_colors.insert(crate::card::ManaColor::Blue);
                    }
                    if s.base.mana_cost.black > 0 && !colors_available.contains(&crate::card::ManaColor::Black) {
                        missing_colors.insert(crate::card::ManaColor::Black);
                    }
                    if s.base.mana_cost.red > 0 && !colors_available.contains(&crate::card::ManaColor::Red) {
                        missing_colors.insert(crate::card::ManaColor::Red);
                    }
                    if s.base.mana_cost.green > 0 && !colors_available.contains(&crate::card::ManaColor::Green) {
                        missing_colors.insert(crate::card::ManaColor::Green);
                    }
                }
                _ => {}
            }
        }

        // Helper: check if land enters tapped
        let enters_tapped = |land: &LandCard| -> bool {
            match land.subtype {
                LandSubtype::Fastland => {
                    let land_count = state
                        .battlefield
                        .permanents()
                        .iter()
                        .filter(|p| matches!(p.card, Card::Land(_)))
                        .count();
                    land_count >= 3
                }
                LandSubtype::Town => state.turn > 3,
                _ => land.enters_tapped,
            }
        };

        // Helper: check if land provides missing color
        let provides_missing_color = |land: &LandCard| -> bool {
            land.colors.iter().any(|c| missing_colors.contains(c))
        };

        // Helper: check if we can cast something this turn with this land
        let can_cast_something_this_turn = |land: &LandCard| -> bool {
            // If land enters tapped, we can't use it this turn
            if enters_tapped(land) {
                return false;
            }

            // What colors would we have after playing this land?
            let mut colors_after = colors_available.clone();
            for color in &land.colors {
                colors_after.insert(*color);
            }

            // Can we cast any spell?
            spells_in_hand.iter().any(|spell| {
                let mv = spell.mana_value();
                if mv > mana_after_land_drop {
                    return false;
                }

                // Check color requirements
                match spell {
                    Card::Creature(c) => {
                        if c.base.mana_cost.white > 0 && !colors_after.contains(&crate::card::ManaColor::White) {
                            return false;
                        }
                        if c.base.mana_cost.blue > 0 && !colors_after.contains(&crate::card::ManaColor::Blue) {
                            return false;
                        }
                        if c.base.mana_cost.black > 0 && !colors_after.contains(&crate::card::ManaColor::Black) {
                            return false;
                        }
                        if c.base.mana_cost.red > 0 && !colors_after.contains(&crate::card::ManaColor::Red) {
                            return false;
                        }
                        if c.base.mana_cost.green > 0 && !colors_after.contains(&crate::card::ManaColor::Green) {
                            return false;
                        }
                        true
                    }
                    Card::Enchantment(e) => {
                        if e.base.mana_cost.white > 0 && !colors_after.contains(&crate::card::ManaColor::White) {
                            return false;
                        }
                        if e.base.mana_cost.blue > 0 && !colors_after.contains(&crate::card::ManaColor::Blue) {
                            return false;
                        }
                        if e.base.mana_cost.black > 0 && !colors_after.contains(&crate::card::ManaColor::Black) {
                            return false;
                        }
                        if e.base.mana_cost.red > 0 && !colors_after.contains(&crate::card::ManaColor::Red) {
                            return false;
                        }
                        if e.base.mana_cost.green > 0 && !colors_after.contains(&crate::card::ManaColor::Green) {
                            return false;
                        }
                        true
                    }
                    Card::Sorcery(s) => {
                        if s.base.mana_cost.white > 0 && !colors_after.contains(&crate::card::ManaColor::White) {
                            return false;
                        }
                        if s.base.mana_cost.blue > 0 && !colors_after.contains(&crate::card::ManaColor::Blue) {
                            return false;
                        }
                        if s.base.mana_cost.black > 0 && !colors_after.contains(&crate::card::ManaColor::Black) {
                            return false;
                        }
                        if s.base.mana_cost.red > 0 && !colors_after.contains(&crate::card::ManaColor::Red) {
                            return false;
                        }
                        if s.base.mana_cost.green > 0 && !colors_after.contains(&crate::card::ManaColor::Green) {
                            return false;
                        }
                        true
                    }
                    _ => true,
                }
            })
        };

        // Sort lands by priority
        let mut sorted_lands = lands.clone();
        sorted_lands.sort_by(|a, b| {
            let a_land = match a.1 {
                Card::Land(l) => l,
                _ => return std::cmp::Ordering::Equal,
            };
            let b_land = match b.1 {
                Card::Land(l) => l,
                _ => return std::cmp::Ordering::Equal,
            };

            let a_tapped = enters_tapped(a_land);
            let b_tapped = enters_tapped(b_land);

            let a_provides_missing = provides_missing_color(a_land);
            let b_provides_missing = provides_missing_color(b_land);

            let a_enables_cast = can_cast_something_this_turn(a_land);
            let b_enables_cast = can_cast_something_this_turn(b_land);

            // PRIORITY 0: Lands that enable casting something this turn
            if a_enables_cast != b_enables_cast {
                return if a_enables_cast {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            // PRIORITY 1: If neither enables casting, prefer lands that provide missing colors
            if !a_enables_cast && !b_enables_cast {
                if a_provides_missing != b_provides_missing {
                    return if a_provides_missing {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }

                // Prefer surveil tapped lands (get value!)
                if a_land.has_surveil != b_land.has_surveil {
                    return if a_land.has_surveil {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }

                // Prefer tapped (save untapped for later)
                if a_tapped != b_tapped {
                    return if a_tapped {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }

                return std::cmp::Ordering::Equal;
            }

            // PRIORITY 2: Both enable casting - prefer surveil for value
            if a_land.has_surveil != b_land.has_surveil {
                return if a_land.has_surveil {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            // Prefer more colors
            b_land.colors.len().cmp(&a_land.colors.len())
        });

        sorted_lands.first().map(|(idx, _)| *idx)
    }

    /// Choose creatures to attack with (all eligible creatures attack)
    pub fn choose_creatures_to_attack(state: &GameState) -> Vec<usize> {
        state
            .battlefield
            .permanents()
            .iter()
            .enumerate()
            .filter_map(|(idx, perm)| {
                if matches!(perm.card, Card::Creature(_)) && !perm.tapped {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Select the best card from a milled set based on game state priorities
    /// This is the exact port of TypeScript selectBestFromMill (lines 1226-1305)
    ///
    /// Priority:
    /// 1. Superior Spider-Man - ALWAYS grab it (key combo piece), unless we already have one
    /// 2. Kiora if Bringer is in hand (need to discard it)
    /// 3. Lands ONLY if we're desperate (0-1 lands on battlefield and none in hand)
    /// 4. Mill enablers (Town Greeter, Overlord, Kiora)
    /// 5. Land if < 4 lands
    /// 6. Any non-combo creature
    /// 7. Any permanent except combo pieces (Bringer, Terror)
    ///
    /// NEVER returns Bringer or Terror - they must stay in graveyard for reanimation
    pub fn select_best_from_mill<'a>(cards: &'a [Card], state: &GameState) -> Option<&'a Card> {
        if cards.is_empty() {
            return None;
        }

        // Calculate game state metrics
        let _has_bringer_in_graveyard = state
            .graveyard
            .cards()
            .iter()
            .any(|c| c.name() == "Bringer of the Last Gift");
        let has_spider_man_in_hand = state
            .hand
            .cards()
            .iter()
            .any(|c| c.name() == "Superior Spider-Man");
        let has_bringer_in_hand = state
            .hand
            .cards()
            .iter()
            .any(|c| c.name() == "Bringer of the Last Gift");

        let land_count = state
            .battlefield
            .permanents()
            .iter()
            .filter(|p| matches!(p.card, Card::Land(_)))
            .count();

        let lands_in_hand = state
            .hand
            .cards()
            .iter()
            .filter(|c| matches!(c, Card::Land(_)))
            .count();

        // Priority 1: Superior Spider-Man - ALWAYS grab it (key combo piece), unless we already have one
        for card in cards {
            if card.name() == "Superior Spider-Man" && !has_spider_man_in_hand {
                return Some(card);
            }
        }

        // Priority 2: Kiora if Bringer is stuck in hand
        for card in cards {
            if card.name() == "Kiora, the Rising Tide" && has_bringer_in_hand {
                return Some(card);
            }
        }

        // Priority 3: Only get land if we're desperate (very few lands and none in hand)
        let desperate_for_land = land_count <= 1 && lands_in_hand == 0;
        if desperate_for_land {
            if let Some(land) = cards.iter().find(|c| matches!(c, Card::Land(_))) {
                return Some(land);
            }
        }

        // Priority 4: Otherwise, get mill enablers (creatures that help us mill more)
        if let Some(enabler) = cards.iter().find(|c| {
            matches!(c, Card::Creature(_))
                && (c.name() == "Town Greeter"
                    || c.name() == "Overlord of the Balemurk"
                    || c.name() == "Kiora, the Rising Tide")
        }) {
            return Some(enabler);
        }

        // Priority 5: Get land if we need it (< 4 lands)
        if land_count < 4 {
            if let Some(land) = cards.iter().find(|c| matches!(c, Card::Land(_))) {
                return Some(land);
            }
        }

        // Priority 6: Get any non-combo creature (but NEVER return Bringer or Terror)
        if let Some(creature) = cards.iter().find(|c| {
            matches!(c, Card::Creature(_))
                && c.name() != "Bringer of the Last Gift"
                && c.name() != "Terror of the Peaks"
        }) {
            return Some(creature);
        }

        // Priority 7: Get any permanent EXCEPT combo pieces (Bringer, Terror)
        // These should stay in the graveyard for reanimation
        cards.iter().find(|c| {
            !matches!(c, Card::Instant(_) | Card::Sorcery(_))
                && c.name() != "Bringer of the Last Gift"
                && c.name() != "Terror of the Peaks"
        })
    }

    /// Choose which card to return from mill
    /// Priority: Spider-Man > Kiora > lands (if desperate) > other creatures > nothing
    pub fn choose_mill_return(graveyard: &[Card], _card_type: CardType) -> Option<usize> {
        // NEVER return Bringer or Terror - they should stay in graveyard
        for (idx, card) in graveyard.iter().enumerate() {
            let name = card.name();
            if name == "Bringer of the Last Gift" || name == "Terror of the Peaks" {
                continue;
            }

            // Prioritize Spider-Man
            if name == "Superior Spider-Man" {
                return Some(idx);
            }
        }

        // Then Kiora
        for (idx, card) in graveyard.iter().enumerate() {
            if card.name() == "Kiora, the Rising Tide" {
                return Some(idx);
            }
        }

        // Then other creatures (but not Bringer/Terror)
        for (idx, card) in graveyard.iter().enumerate() {
            if matches!(card, Card::Creature(_))
                && card.name() != "Bringer of the Last Gift"
                && card.name() != "Terror of the Peaks"
            {
                return Some(idx);
            }
        }

        None
    }

    /// Choose which card to discard
    /// Discard non-essentials: lands > expensive spells > creatures
    pub fn choose_discard(hand: &[Card]) -> Option<usize> {
        // NEVER discard Bringer or Terror - they're combo pieces
        // Prefer to discard lands
        for (idx, card) in hand.iter().enumerate() {
            let name = card.name();
            if matches!(card, Card::Land(_))
                && name != "Bringer of the Last Gift"
                && name != "Terror of the Peaks"
            {
                return Some(idx);
            }
        }

        // Then expensive spells (but not combo pieces)
        for (idx, card) in hand.iter().enumerate() {
            let name = card.name();
            if card.mana_value() >= 4
                && name != "Bringer of the Last Gift"
                && name != "Terror of the Peaks"
            {
                return Some(idx);
            }
        }

        // Last resort: any card that's not a combo piece
        for (idx, card) in hand.iter().enumerate() {
            let name = card.name();
            if name != "Bringer of the Last Gift" && name != "Terror of the Peaks" {
                return Some(idx);
            }
        }

        None
    }

    /// Check if the combo is ready to win
    /// Combo: Spider-Man + Bringer in graveyard + 4+ mana available
    pub fn is_combo_ready(state: &GameState) -> bool {
        let has_spider_man_in_hand = state
            .hand
            .cards()
            .iter()
            .any(|c| c.name() == "Superior Spider-Man");

        let has_bringer_in_graveyard = state
            .graveyard
            .cards()
            .iter()
            .any(|c| c.name() == "Bringer of the Last Gift");

        let has_enough_mana = state.mana_pool.total() >= 4;

        has_spider_man_in_hand && has_bringer_in_graveyard && has_enough_mana
    }

    /// Check if Terror of the Peaks is in play (damage trigger)
    pub fn has_terror_in_play(state: &GameState) -> bool {
        state
            .battlefield
            .permanents()
            .iter()
            .any(|p| p.card.name() == "Terror of the Peaks")
    }

    /// Check if Bringer is in play (mass reanimate trigger)
    pub fn has_bringer_in_play(state: &GameState) -> bool {
        state
            .battlefield
            .permanents()
            .iter()
            .any(|p| p.card.name() == "Bringer of the Last Gift")
    }

    // --- Helper functions ---

    fn count_lands(hand: &[Card]) -> usize {
        hand.iter().filter(|c| matches!(c, Card::Land(_))).count()
    }

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

    fn is_playable_early_spell(card: &Card) -> bool {
        card.mana_value() <= 3 && !matches!(card, Card::Land(_))
    }

    fn can_cast(card: &Card, mana_pool: &crate::game::mana::ManaPool) -> bool {
        use crate::card::Card::*;
        let cost = match card {
            Creature(c) => &c.base.mana_cost,
            Instant(c) => &c.base.mana_cost,
            Sorcery(c) => &c.base.mana_cost,
            Enchantment(c) => &c.base.mana_cost,
            Saga(c) => &c.base.mana_cost,
            Land(_) => return true,
        };
        mana_pool.can_pay(cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardDatabase;

    #[test]
    fn test_should_mulligan_bad_hand() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let bad_hand = vec![
            forest.clone(),
            terror.clone(),
            terror.clone(),
            terror.clone(),
            terror.clone(),
            terror.clone(),
            terror.clone(),
        ];
        assert!(DecisionEngine::should_mulligan(&bad_hand, 0));
    }

    #[test]
    fn test_should_keep_with_enabler() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");

        let hand = vec![
            forest.clone(),
            town_greeter.clone(),
            forest.clone(),
            forest.clone(),
            forest.clone(),
            forest.clone(),
            forest.clone(),
        ];
        assert!(!DecisionEngine::should_mulligan(&hand, 0));
    }

    #[test]
    fn test_choose_mill_return_prioritizes_spider_man() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let spider_man = db
            .get_card("Superior Spider-Man")
            .expect("Superior Spider-Man should exist");
        let kiora = db
            .get_card("Kiora, the Rising Tide")
            .expect("Kiora should exist");

        let graveyard = vec![kiora.clone(), spider_man.clone()];
        let choice = DecisionEngine::choose_mill_return(&graveyard, CardType::Creature);

        // Should choose Spider-Man (index 1)
        assert_eq!(choice, Some(1));
    }

    #[test]
    fn test_choose_mill_return_never_returns_bringer() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let bringer = db
            .get_card("Bringer of the Last Gift")
            .expect("Bringer should exist");
        let kiora = db
            .get_card("Kiora, the Rising Tide")
            .expect("Kiora should exist");

        let graveyard = vec![bringer.clone(), kiora.clone()];
        let choice = DecisionEngine::choose_mill_return(&graveyard, CardType::Creature);

        // Should choose Kiora (index 1), never Bringer
        assert_eq!(choice, Some(1));
    }

    #[test]
    fn test_choose_discard_prefers_lands() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let hand = vec![forest.clone(), terror.clone()];
        let choice = DecisionEngine::choose_discard(&hand);

        // Should choose land (index 0)
        assert_eq!(choice, Some(0));
    }

    #[test]
    fn test_choose_discard_never_discards_bringer() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let bringer = db
            .get_card("Bringer of the Last Gift")
            .expect("Bringer should exist");
        let forest = db.get_card("Forest").expect("Forest should exist");

        let hand = vec![bringer.clone(), forest.clone()];
        let choice = DecisionEngine::choose_discard(&hand);

        // Should choose land (index 1), never Bringer
        assert_eq!(choice, Some(1));
    }

    #[test]
    fn test_select_best_from_mill_empty() {
        let state = GameState::new();
        let cards: Vec<Card> = vec![];
        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        assert!(choice.is_none());
    }

    #[test]
    fn test_select_best_from_mill_priority_1_spider_man() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let spider_man = db
            .get_card("Superior Spider-Man")
            .expect("Superior Spider-Man should exist");
        let kiora = db
            .get_card("Kiora, the Rising Tide")
            .expect("Kiora should exist");

        let state = GameState::new();
        let cards = vec![kiora.clone(), spider_man.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose Spider-Man (Priority 1)
        assert_eq!(choice.map(|c| c.name()), Some("Superior Spider-Man"));
    }

    #[test]
    fn test_select_best_from_mill_priority_1_spider_man_already_in_hand() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let spider_man = db
            .get_card("Superior Spider-Man")
            .expect("Superior Spider-Man should exist");
        let kiora = db
            .get_card("Kiora, the Rising Tide")
            .expect("Kiora should exist");

        let mut state = GameState::new();
        state.hand.add_card(spider_man.clone());

        let cards = vec![kiora.clone(), spider_man.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose Kiora since Spider-Man already in hand
        assert_eq!(choice.map(|c| c.name()), Some("Kiora, the Rising Tide"));
    }

    #[test]
    fn test_select_best_from_mill_priority_2_kiora_with_bringer_in_hand() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let kiora = db
            .get_card("Kiora, the Rising Tide")
            .expect("Kiora should exist");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let bringer = db
            .get_card("Bringer of the Last Gift")
            .expect("Bringer should exist");

        let state = GameState::new();
        let mut state = state;
        state.hand.add_card(bringer.clone());

        let cards = vec![forest.clone(), kiora.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose Kiora (Priority 2) because Bringer in hand
        assert_eq!(choice.map(|c| c.name()), Some("Kiora, the Rising Tide"));
    }

    #[test]
    fn test_select_best_from_mill_priority_3_desperate_for_land() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let state = GameState::new();
        // 0 lands on battlefield, 0 lands in hand = desperate
        let cards = vec![terror.clone(), forest.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose land (Priority 3) because desperate
        assert_eq!(choice.map(|c| c.name()), Some("Forest"));
    }

    #[test]
    fn test_select_best_from_mill_priority_3_not_desperate() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");

        let mut state = GameState::new();
        // Add 2 lands to battlefield (not desperate)
        let land1 = Card::Land(crate::card::LandCard {
            base: crate::card::BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: crate::card::LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![crate::card::ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });
        let perm1 = crate::game::zones::Permanent::new(land1.clone(), 0);
        state.battlefield.add_permanent(perm1);
        let perm2 = crate::game::zones::Permanent::new(land1, 0);
        state.battlefield.add_permanent(perm2);

        let cards = vec![forest.clone(), town_greeter.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose Town Greeter (Priority 4) not land
        assert_eq!(choice.map(|c| c.name()), Some("Town Greeter"));
    }

    #[test]
    fn test_select_best_from_mill_priority_4_mill_enablers() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");
        let overlord = db
            .get_card("Overlord of the Balemurk")
            .expect("Overlord should exist");
        let forest = db.get_card("Forest").expect("Forest should exist");

        let state = GameState::new();
        // Add 2 lands to battlefield (not desperate)
        let mut state = state;
        let land1 = Card::Land(crate::card::LandCard {
            base: crate::card::BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: crate::card::LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![crate::card::ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });
        let perm1 = crate::game::zones::Permanent::new(land1.clone(), 0);
        state.battlefield.add_permanent(perm1);
        let perm2 = crate::game::zones::Permanent::new(land1, 0);
        state.battlefield.add_permanent(perm2);

        let cards = vec![forest.clone(), town_greeter.clone(), overlord.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose Town Greeter (Priority 4 - first enabler found)
        assert_eq!(choice.map(|c| c.name()), Some("Town Greeter"));
    }

    #[test]
    fn test_select_best_from_mill_priority_5_land_if_less_than_4() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let forest = db.get_card("Forest").expect("Forest should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let state = GameState::new();
        let mut state = state;
        // Add 3 lands to battlefield (< 4)
        let land1 = Card::Land(crate::card::LandCard {
            base: crate::card::BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: crate::card::LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![crate::card::ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });
        for _ in 0..3 {
            let perm = crate::game::zones::Permanent::new(land1.clone(), 0);
            state.battlefield.add_permanent(perm);
        }

        let cards = vec![terror.clone(), forest.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose land (Priority 5) because < 4 lands
        assert_eq!(choice.map(|c| c.name()), Some("Forest"));
    }

    #[test]
    fn test_select_best_from_mill_priority_6_non_combo_creature() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let town_greeter = db.get_card("Town Greeter").expect("Town Greeter should exist");
        let bringer = db
            .get_card("Bringer of the Last Gift")
            .expect("Bringer should exist");

        let state = GameState::new();
        let mut state = state;
        // Add 4+ lands to battlefield (not desperate, don't need more)
        let land1 = Card::Land(crate::card::LandCard {
            base: crate::card::BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: crate::card::LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![crate::card::ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });
        for _ in 0..4 {
            let perm = crate::game::zones::Permanent::new(land1.clone(), 0);
            state.battlefield.add_permanent(perm);
        }

        let cards = vec![bringer.clone(), town_greeter.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose Town Greeter (Priority 6) not Bringer
        assert_eq!(choice.map(|c| c.name()), Some("Town Greeter"));
    }

    #[test]
    fn test_select_best_from_mill_never_returns_bringer() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let bringer = db
            .get_card("Bringer of the Last Gift")
            .expect("Bringer should exist");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");

        let state = GameState::new();
        let mut state = state;
        // Add 4+ lands to battlefield
        let land1 = Card::Land(crate::card::LandCard {
            base: crate::card::BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: crate::card::LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![crate::card::ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });
        for _ in 0..4 {
            let perm = crate::game::zones::Permanent::new(land1.clone(), 0);
            state.battlefield.add_permanent(perm);
        }

        let cards = vec![bringer.clone(), terror.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should return None - never Bringer or Terror
        assert!(choice.is_none());
    }

    #[test]
    fn test_select_best_from_mill_never_returns_terror() {
        let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
        let terror = db.get_card("Terror of the Peaks").expect("Terror should exist");
        let forest = db.get_card("Forest").expect("Forest should exist");

        let state = GameState::new();
        let mut state = state;
        // Add 4+ lands to battlefield
        let land1 = Card::Land(crate::card::LandCard {
            base: crate::card::BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: crate::card::LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![crate::card::ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });
        for _ in 0..4 {
            let perm = crate::game::zones::Permanent::new(land1.clone(), 0);
            state.battlefield.add_permanent(perm);
        }

        let cards = vec![terror.clone(), forest.clone()];

        let choice = DecisionEngine::select_best_from_mill(&cards, &state);
        // Should choose forest, never Terror
        assert_eq!(choice.map(|c| c.name()), Some("Forest"));
    }
}

