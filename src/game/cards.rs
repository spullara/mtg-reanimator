use crate::card::{Card, CardDatabase, LandSubtype, ManaColor};
use crate::game::mana::ManaPool;
use crate::game::state::GameState;
use crate::game::zones::{CounterType, Permanent};

/// Check if a card can be cast with the current mana pool
pub fn can_cast(card: &Card, mana_pool: &ManaPool) -> bool {
    let cost = match card {
        Card::Creature(c) => &c.base.mana_cost,
        Card::Instant(c) => &c.base.mana_cost,
        Card::Sorcery(c) => &c.base.mana_cost,
        Card::Enchantment(c) => &c.base.mana_cost,
        Card::Saga(c) => &c.base.mana_cost,
        Card::Land(_) => return true, // Lands don't require mana
    };

    mana_pool.can_pay(cost)
}

/// Play a land from hand to battlefield with proper tapping logic
pub fn play_land(state: &mut GameState, card: &Card) -> Result<(), String> {
    let land = match card {
        Card::Land(l) => l,
        _ => return Err("Not a land card".to_string()),
    };

    // Determine if land enters tapped
    let mut enters_tapped = land.enters_tapped;

    // Conditional tapping logic based on land subtype
    match land.subtype {
        LandSubtype::Shock => {
            // Shock lands can pay 2 life to enter untapped
            // For now, simplified: always enter untapped (decision logic in simulation)
            enters_tapped = false;
        }
        LandSubtype::Fastland => {
            // Enter untapped if you control 2 or fewer other lands
            let land_count = state
                .battlefield
                .permanents()
                .iter()
                .filter(|p| matches!(p.card, Card::Land(_)))
                .count();
            enters_tapped = land_count >= 3;
        }
        LandSubtype::Town => {
            // Starting Town: enter untapped, tap after turn 3
            enters_tapped = false;
        }
        LandSubtype::Utility => {
            // Verge lands: enter untapped if revealed land type
            // Simplified: always enter untapped
            enters_tapped = false;
        }
        _ => {} // Basic, Surveil use enters_tapped from card definition
    }

    let mut permanent = Permanent::new(card.clone(), state.turn);
    permanent.tapped = enters_tapped;

    // Handle surveil lands
    if land.has_surveil && land.surveil_amount > 0 {
        // Surveil logic: mill and choose what to put back
        let milled = state.library.mill(land.surveil_amount as usize);
        for card in milled {
            state.graveyard.add_card(card);
        }
    }

    state.battlefield.add_permanent(permanent);
    state.land_played_this_turn = true;

    Ok(())
}

/// Tap a land to add mana to the pool
pub fn tap_land_for_mana(permanent: &Permanent, mana_pool: &mut ManaPool) -> Result<(), String> {
    if permanent.tapped {
        return Err("Land is already tapped".to_string());
    }

    let land = match &permanent.card {
        Card::Land(l) => l,
        _ => return Err("Not a land card".to_string()),
    };

    // Add mana based on land colors
    for color in &land.colors {
        match color {
            ManaColor::White => mana_pool.add_mana('W', 1),
            ManaColor::Blue => mana_pool.add_mana('U', 1),
            ManaColor::Black => mana_pool.add_mana('B', 1),
            ManaColor::Red => mana_pool.add_mana('R', 1),
            ManaColor::Green => mana_pool.add_mana('G', 1),
            ManaColor::Colorless => mana_pool.add_mana('C', 1),
        }
    }

    Ok(())
}

/// Cast a spell and resolve its effects
pub fn cast_spell(
    state: &mut GameState,
    card: &Card,
    _db: &CardDatabase,
) -> Result<(), String> {
    match card {
        Card::Instant(spell) | Card::Sorcery(spell) => {
            // Process instant/sorcery abilities
            for ability in &spell.abilities {
                match ability.as_str() {
                    "mill_4_return_permanent" => {
                        // Cache Grab: mill 4, return permanent
                        let milled = state.library.mill(4);
                        for card in milled {
                            state.graveyard.add_card(card);
                        }
                    }
                    "search_land_or_creature_with_evidence" => {
                        // Analyze the Pollen: evidence 8, search
                        // Simplified: just mill to represent searching
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        Card::Enchantment(spell) => {
            // Process enchantment abilities
            for ability in &spell.abilities {
                match ability.as_str() {
                    "etb_mill_4_return_artifact_creature_land" => {
                        // Dredger's Insight: mill 4, return artifact/creature/land
                        let milled = state.library.mill(4);
                        for card in milled {
                            state.graveyard.add_card(card);
                        }
                    }
                    "graveyard_leave_lifegain" => {
                        // Dredger's Insight: gain life when leaving graveyard
                        // This is a triggered ability, handled elsewhere
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        Card::Saga(saga) => {
            // Sagas enter with 1 lore counter
            state.saga_counters.insert(saga.base.name.clone(), 1);
            Ok(())
        }
        _ => Err("Not a spell card".to_string()),
    }
}

/// Advance saga to next chapter and resolve chapter ability
pub fn advance_saga(state: &mut GameState, saga_name: &str) -> Result<(), String> {
    let current_counters = state.saga_counters.get(saga_name).copied().unwrap_or(0);
    let new_counters = current_counters + 1;
    state.saga_counters.insert(saga_name.to_string(), new_counters);

    // Resolve chapter ability based on saga name and chapter number
    match saga_name {
        "Awaken the Honored Dead" => {
            match new_counters {
                1 => {
                    // Chapter 1: Destroy target nonland permanent
                    // Simplified: destroy first nonland permanent on battlefield
                    if let Some(pos) = state
                        .battlefield
                        .permanents()
                        .iter()
                        .position(|p| !matches!(p.card, Card::Land(_)))
                    {
                        state.battlefield.remove_permanent(pos);
                    }
                }
                2 => {
                    // Chapter 2: Mill 3
                    let milled = state.library.mill(3);
                    for card in milled {
                        state.graveyard.add_card(card);
                    }
                }
                3 => {
                    // Chapter 3: Discard a card, return creature or land from graveyard
                    if state.hand.size() > 0 {
                        state.hand.remove_card(0);
                    }
                    // Return first creature or land from graveyard
                    if let Some(card) = state.graveyard.find_creature() {
                        if let Some(pos) = state
                            .graveyard
                            .cards()
                            .iter()
                            .position(|c| c.name() == card.name())
                        {
                            if let Some(returned) = state.graveyard.remove_card(pos) {
                                let perm = Permanent::new(returned, state.turn);
                                state.battlefield.add_permanent(perm);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}

/// Process enter-the-battlefield triggers for a creature
pub fn process_etb_triggers(
    state: &mut GameState,
    permanent: &mut Permanent,
    _db: &CardDatabase,
) -> Result<(), String> {
    // Extract abilities before borrowing permanent mutably
    let abilities = match &permanent.card {
        Card::Creature(c) => c.abilities.clone(),
        _ => return Ok(()), // Not a creature
    };

    // Process abilities
    for ability in abilities {
        match ability.as_str() {
            "etb_mill_4_return_land" => {
                // Town Greeter: mill 4, may return land
                let milled = state.library.mill(4);
                for card in milled {
                    state.graveyard.add_card(card);
                }
            }
            "etb_draw_2_discard_2" => {
                // Kiora: draw 2, discard 2
                for _ in 0..2 {
                    state.draw_card();
                }
                // Discard 2 (simplified: just remove from hand)
                if state.hand.size() >= 2 {
                    state.hand.remove_card(0);
                    state.hand.remove_card(0);
                }
            }
            "impending_5" => {
                // Overlord: enters with 5 time counters
                permanent.add_counter(CounterType::Time, 5);
            }
            "etb_damage_trigger" => {
                // Terror of the Peaks: damage trigger (setup, actual damage on creature ETB)
                // This is a triggered ability that fires when other creatures enter
                // Stored for later trigger resolution
            }
            "etb_mass_reanimate" => {
                // Bringer of the Last Gift: mass reanimate
                // Return all creature cards from graveyard to battlefield
                let graveyard_cards = state.graveyard.cards().to_vec();
                for card in graveyard_cards {
                    if matches!(card, Card::Creature(_)) {
                        let perm = Permanent::new(card.clone(), state.turn);
                        state.battlefield.add_permanent(perm);
                    }
                }
                // Clear graveyard of creatures
                state.graveyard.clear_creatures();
            }
            "etb_or_attack_mill_4_return" => {
                // Overlord of the Balemurk: mill 4, return creature or land
                let milled = state.library.mill(4);
                for card in milled {
                    state.graveyard.add_card(card);
                }
            }
            "mind_swap_copy" => {
                // Superior Spider-Man: copy creature from graveyard
                // Find a creature in graveyard and copy it
                if let Some(creature) = state.graveyard.find_creature() {
                    permanent.is_copy_of = Some(creature.name().to_string());
                }
            }
            _ => {} // Other abilities handled elsewhere
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{BaseCard, CreatureCard, LandCard, LandSubtype, ManaCost, ManaColor};

    #[test]
    fn test_can_cast_with_sufficient_mana() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 2);
        pool.add_mana('U', 1);

        let card = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test".to_string(),
                mana_cost: ManaCost {
                    white: 1,
                    generic: 1,
                    ..Default::default()
                },
                mana_value: 2,
            },
            power: 2,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        assert!(can_cast(&card, &pool));
    }

    #[test]
    fn test_play_basic_land() {
        let mut state = GameState::new();
        let land = Card::Land(LandCard {
            base: BaseCard {
                name: "Forest".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 0,
            },
            subtype: LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });

        let result = play_land(&mut state, &land);
        assert!(result.is_ok());
        assert_eq!(state.battlefield.size(), 1);
        assert!(!state.battlefield.permanents()[0].tapped);
    }

    #[test]
    fn test_play_fastland_with_few_lands() {
        let mut state = GameState::new();
        let fastland = Card::Land(LandCard {
            base: BaseCard {
                name: "Blooming Marsh".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 0,
            },
            subtype: LandSubtype::Fastland,
            enters_tapped: false,
            colors: vec![ManaColor::Black, ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });

        let result = play_land(&mut state, &fastland);
        assert!(result.is_ok());
        assert!(!state.battlefield.permanents()[0].tapped);
    }

    #[test]
    fn test_play_fastland_with_many_lands() {
        let mut state = GameState::new();

        // Add 3 lands to battlefield
        for _ in 0..3 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: "Forest".to_string(),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            let perm = Permanent::new(land, 1);
            state.battlefield.add_permanent(perm);
        }

        let fastland = Card::Land(LandCard {
            base: BaseCard {
                name: "Blooming Marsh".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 0,
            },
            subtype: LandSubtype::Fastland,
            enters_tapped: false,
            colors: vec![ManaColor::Black, ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });

        let result = play_land(&mut state, &fastland);
        assert!(result.is_ok());
        assert!(state.battlefield.permanents()[3].tapped);
    }

    #[test]
    fn test_tap_land_for_mana() {
        let land = Card::Land(LandCard {
            base: BaseCard {
                name: "Forest".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 0,
            },
            subtype: LandSubtype::Basic,
            enters_tapped: false,
            colors: vec![ManaColor::Green],
            has_surveil: false,
            surveil_amount: 0,
        });

        let perm = Permanent::new(land, 1);
        let mut pool = ManaPool::new();

        let result = tap_land_for_mana(&perm, &mut pool);
        assert!(result.is_ok());
        assert_eq!(pool.green, 1);
    }

    #[test]
    fn test_advance_saga_chapter_1() {
        let mut state = GameState::new();

        // Add a nonland permanent to destroy
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 1,
            },
            power: 1,
            toughness: 1,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        let perm = Permanent::new(creature, 1);
        state.battlefield.add_permanent(perm);

        let result = advance_saga(&mut state, "Awaken the Honored Dead");
        assert!(result.is_ok());
        assert_eq!(state.saga_counters.get("Awaken the Honored Dead"), Some(&1));
        assert_eq!(state.battlefield.size(), 0); // Creature destroyed
    }

    #[test]
    fn test_advance_saga_chapter_2() {
        let mut state = GameState::new();
        state.saga_counters.insert("Awaken the Honored Dead".to_string(), 1);

        // Add cards to library
        for _ in 0..5 {
            let card = Card::Land(LandCard {
                base: BaseCard {
                    name: "Forest".to_string(),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.library.add_card(card);
        }

        let result = advance_saga(&mut state, "Awaken the Honored Dead");
        assert!(result.is_ok());
        assert_eq!(state.saga_counters.get("Awaken the Honored Dead"), Some(&2));
        assert_eq!(state.graveyard.size(), 3); // 3 cards milled
    }
}

