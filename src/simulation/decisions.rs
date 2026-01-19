use crate::card::{Card, CardDatabase, CardType};
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

    /// Choose which card to play from hand
    /// Priority: mill enablers > creatures > lands
    pub fn choose_card_to_play(
        hand: &[Card],
        state: &GameState,
        _db: &CardDatabase,
    ) -> Option<usize> {
        // First, try to play a mill enabler
        for (idx, card) in hand.iter().enumerate() {
            if Self::is_mill_enabler(card) && Self::can_cast(card, &state.mana_pool) {
                return Some(idx);
            }
        }

        // Then, try to play a creature
        for (idx, card) in hand.iter().enumerate() {
            if matches!(card, Card::Creature(_)) && Self::can_cast(card, &state.mana_pool) {
                return Some(idx);
            }
        }

        // Finally, try to play a land if we haven't played one this turn
        if !state.land_played_this_turn {
            for (idx, card) in hand.iter().enumerate() {
                if matches!(card, Card::Land(_)) {
                    return Some(idx);
                }
            }
        }

        None
    }

    /// Choose which land to play
    /// Prefer: untapped > dual lands > basic lands
    pub fn choose_land_to_play(hand: &[Card], _state: &GameState) -> Option<usize> {
        let mut best_idx = None;
        let mut best_score = -1i32;

        for (idx, card) in hand.iter().enumerate() {
            if let Card::Land(land) = card {
                // Score: untapped lands get +10, dual lands get +5
                let mut score = 0i32;
                if !land.enters_tapped {
                    score += 10;
                }
                if land.colors.len() > 1 {
                    score += 5;
                }

                if score > best_score {
                    best_score = score;
                    best_idx = Some(idx);
                }
            }
        }

        best_idx
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
}

