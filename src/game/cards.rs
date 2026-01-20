use crate::card::{Card, CardDatabase, CardType, LandSubtype, ManaColor};
use crate::game::mana::ManaPool;
use crate::game::state::GameState;
use crate::game::zones::{CounterType, Permanent};
use crate::simulation::decisions::DecisionEngine;

/// Check if a creature has impending counters (enters as enchantment)
pub fn has_impending(card: &Card) -> bool {
    match card {
        Card::Creature(c) => c.impending_counters.is_some(),
        _ => false,
    }
}

/// Get impending counter count for a creature
pub fn get_impending_counters(card: &Card) -> u32 {
    match card {
        Card::Creature(c) => c.impending_counters.unwrap_or(0),
        _ => 0,
    }
}

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
        resolve_surveil(state, land.surveil_amount as usize, false);
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

/// Cast a creature, handling impending logic
pub fn cast_creature(
    state: &mut GameState,
    card: &Card,
    use_impending: bool,
) -> Result<(), String> {
    match card {
        Card::Creature(_) => {},
        _ => return Err("Not a creature card".to_string()),
    };

    let mut permanent = Permanent::new(card.clone(), state.turn);

    // Handle impending creatures
    if use_impending && has_impending(card) {
        let counters = get_impending_counters(card);
        permanent.add_counter(CounterType::Time, counters);
    }

    state.battlefield.add_permanent(permanent);
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
                        // Dredger's Insight: mill 4, return artifact/creature/land to hand
                        let milled = state.library.mill(4);
                        let mut milled_cards = Vec::new();
                        for card in milled {
                            milled_cards.push(card);
                        }

                        // Choose which card to return (prioritize Spider-Man, then Kiora, then lands)
                        if let Some(idx) = DecisionEngine::choose_mill_return(&milled_cards, CardType::Creature) {
                            let card_to_return = milled_cards.remove(idx);
                            state.hand.add_card(card_to_return);
                        }

                        // Rest go to graveyard
                        for card in milled_cards {
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

/// Resolve surveil mechanic: look at top N cards and decide which go to graveyard
///
/// EXACT LOGIC FROM TYPESCRIPT:
/// - Check hasKioraInHand INSIDE the loop (it can change)
/// - Only remove from library if putting in graveyard
/// - If keeping on top, do NOT touch the library - leave card in place
pub fn resolve_surveil(state: &mut GameState, count: usize, verbose: bool) {
    let mut to_graveyard: Vec<String> = Vec::new();
    let mut to_top: Vec<String> = Vec::new();

    for _ in 0..count {
        // Check if library is empty
        if state.library.is_empty() {
            break;
        }

        // Peek at top card without removing it
        if let Some(top_card) = state.library.peek_top() {
            let card_name = top_card.name().to_string();

            // Decision: keep on top or put in graveyard?
            // Graveyard: Bringer, Terror, Overlord (want to reanimate these)
            // Also put Kiora if we already have one (for reanimation value)
            // Top: Spider-Man (MUST stay in hand!), lands, mill spells
            let has_kiora_in_hand = state.hand.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide");
            let put_in_graveyard = card_name == "Bringer of the Last Gift"
                || card_name == "Terror of the Peaks"
                || card_name == "Overlord of the Balemurk"
                || (card_name == "Kiora, the Rising Tide" && has_kiora_in_hand)
                || card_name == "Town Greeter"; // Cheap 1/1, better to reanimate than draw

            if put_in_graveyard {
                // Remove from library and add to graveyard
                if let Some(card) = state.library.draw() {
                    state.graveyard.add_card(card);
                    to_graveyard.push(card_name);
                }
            } else {
                // Keep on top - do NOT touch the library
                to_top.push(card_name);
            }
        }
    }

    if verbose && (!to_graveyard.is_empty() || !to_top.is_empty()) {
        if !to_graveyard.is_empty() {
            println!("    Surveil -> graveyard: {}", to_graveyard.join(", "));
        }
        if !to_top.is_empty() {
            println!("    Surveil -> kept on top: {}", to_top.join(", "));
        }
    }
}

/// Resolve Kiora's ETB ability: draw 2, discard 2
///
/// EXACT LOGIC FROM TYPESCRIPT:
/// - Draw 2 cards first
/// - Then discard 2 cards with 4-priority system:
///   1. Bringer of the Last Gift
///   2. Terror of the Peaks
///   3. Excess lands (only if > 2 lands in hand)
///   4. Last card in hand
/// - Each discard iteration searches for the best card independently
pub fn resolve_kiora_etb(state: &mut GameState, verbose: bool) {
    // Draw 2, discard 2
    let hand_before = state.hand.size();
    state.draw_card();
    state.draw_card();

    // Collect drawn cards for logging
    let drawn: Vec<String> = state.hand.cards()
        .iter()
        .skip(hand_before)
        .map(|c| c.name().to_string())
        .collect();

    if verbose {
        println!("    Kiora ETB: drew {}", drawn.join(", "));
    }

    // Discard 2 - prioritize discarding Bringer/Terror
    let mut discarded: Vec<String> = Vec::new();
    for _ in 0..2 {
        if state.hand.size() == 0 {
            break;
        }

        // Find best card to discard
        let mut to_discard_idx: Option<usize> = None;

        // Priority 1: Bringer of the Last Gift
        if to_discard_idx.is_none() {
            to_discard_idx = state.hand.cards()
                .iter()
                .position(|c| c.name() == "Bringer of the Last Gift");
        }

        // Priority 2: Terror of the Peaks
        if to_discard_idx.is_none() {
            to_discard_idx = state.hand.cards()
                .iter()
                .position(|c| c.name() == "Terror of the Peaks");
        }

        // Priority 3: Excess lands (only if > 2 lands in hand)
        if to_discard_idx.is_none() {
            let lands: Vec<usize> = state.hand.cards()
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Card::Land(_)))
                .map(|(i, _)| i)
                .collect();
            if lands.len() > 2 {
                // Take the last land
                to_discard_idx = lands.last().copied();
            }
        }

        // Priority 4: Last card in hand
        if to_discard_idx.is_none() {
            to_discard_idx = Some(state.hand.size() - 1);
        }

        // Discard the card
        if let Some(idx) = to_discard_idx {
            if let Some(card) = state.hand.remove_card(idx) {
                let card_name = card.name().to_string();
                state.graveyard.add_card(card);
                discarded.push(card_name);
            }
        }
    }

    if verbose {
        println!("    Kiora ETB: discarded {}", discarded.join(", "));
    }
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

    #[test]
    fn test_has_impending() {
        let impending_creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Overlord".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 5,
            },
            power: 5,
            toughness: 5,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: Some(ManaCost::default()),
            impending_counters: Some(5),
        });

        let normal_creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test".to_string(),
                mana_cost: ManaCost::default(),
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

        assert!(has_impending(&impending_creature));
        assert!(!has_impending(&normal_creature));
    }

    #[test]
    fn test_get_impending_counters() {
        let impending_creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Overlord".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 5,
            },
            power: 5,
            toughness: 5,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: Some(ManaCost::default()),
            impending_counters: Some(5),
        });

        assert_eq!(get_impending_counters(&impending_creature), 5);
    }

    #[test]
    fn test_cast_creature_with_impending() {
        let mut state = GameState::new();
        let impending_creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Overlord".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 5,
            },
            power: 5,
            toughness: 5,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: Some(ManaCost::default()),
            impending_counters: Some(5),
        });

        let result = cast_creature(&mut state, &impending_creature, true);
        assert!(result.is_ok());
        assert_eq!(state.battlefield.size(), 1);
        let perm = &state.battlefield.permanents()[0];
        assert_eq!(perm.get_counter(CounterType::Time), 5);
    }

    #[test]
    fn test_cast_creature_without_impending() {
        let mut state = GameState::new();
        let impending_creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Overlord".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 5,
            },
            power: 5,
            toughness: 5,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: Some(ManaCost::default()),
            impending_counters: Some(5),
        });

        let result = cast_creature(&mut state, &impending_creature, false);
        assert!(result.is_ok());
        assert_eq!(state.battlefield.size(), 1);
        let perm = &state.battlefield.permanents()[0];
        assert_eq!(perm.get_counter(CounterType::Time), 0);
    }

    #[test]
    fn test_resolve_surveil_puts_bringer_in_graveyard() {
        let mut state = GameState::new();

        // Add Bringer to library
        let bringer = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 8,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        state.library.add_card(bringer);

        // Add a land to library
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
        state.library.add_card(land);

        // Surveil 1
        resolve_surveil(&mut state, 1, false);

        // Bringer should be in graveyard
        assert_eq!(state.graveyard.size(), 1);
        assert_eq!(state.graveyard.cards()[0].name(), "Bringer of the Last Gift");

        // Land should still be in library
        assert_eq!(state.library.size(), 1);
        assert_eq!(state.library.peek_top().unwrap().name(), "Forest");
    }

    #[test]
    fn test_resolve_surveil_keeps_land_on_top() {
        let mut state = GameState::new();

        // Add a land to library
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
        state.library.add_card(land);

        // Add another card below
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
        state.library.add_card(creature);

        // Surveil 1
        resolve_surveil(&mut state, 1, false);

        // Graveyard should be empty
        assert_eq!(state.graveyard.size(), 0);

        // Land should still be on top of library
        assert_eq!(state.library.size(), 2);
        assert_eq!(state.library.peek_top().unwrap().name(), "Forest");
    }

    #[test]
    fn test_resolve_kiora_etb_draws_2_cards() {
        let mut state = GameState::new();

        // Add 4 cards to library (2 to draw, 2 to keep in hand after discard)
        for i in 0..4 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: format!("Forest {}", i),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.library.add_card(land);
        }

        // Add 2 cards to hand to keep after discard
        for i in 0..2 {
            let creature = Card::Creature(CreatureCard {
                base: BaseCard {
                    name: format!("Creature {}", i),
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
            state.hand.add_card(creature);
        }

        let hand_before = state.hand.size();
        resolve_kiora_etb(&mut state, false);

        // Should have drawn 2 cards and discarded 2 cards (net: +0)
        // But we had 2 creatures in hand, so: 2 + 2 (drawn) - 2 (discarded) = 2
        assert_eq!(state.hand.size(), hand_before);
        assert_eq!(state.library.size(), 2); // 4 - 2 drawn
        assert_eq!(state.graveyard.size(), 2); // 2 discarded
    }

    #[test]
    fn test_resolve_kiora_etb_discards_bringer_first() {
        let mut state = GameState::new();

        // Add 2 cards to library to draw
        for i in 0..2 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: format!("Forest {}", i),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.library.add_card(land);
        }

        // Add Bringer to hand
        let bringer = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 8,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        state.hand.add_card(bringer);

        resolve_kiora_etb(&mut state, false);

        // Should have discarded 2 cards (Bringer + 1 other)
        assert_eq!(state.graveyard.size(), 2);
        // Bringer should be in graveyard
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift"));
    }

    #[test]
    fn test_resolve_kiora_etb_discards_terror_second() {
        let mut state = GameState::new();

        // Add 2 cards to library to draw
        for i in 0..2 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: format!("Forest {}", i),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.library.add_card(land);
        }

        // Add Terror to hand (no Bringer)
        let terror = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Terror of the Peaks".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 5,
            },
            power: 5,
            toughness: 5,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        state.hand.add_card(terror);

        resolve_kiora_etb(&mut state, false);

        // Should have discarded 2 cards (Terror + 1 other)
        assert_eq!(state.graveyard.size(), 2);
        // Terror should be in graveyard
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Terror of the Peaks"));
    }

    #[test]
    fn test_resolve_kiora_etb_discards_excess_lands() {
        let mut state = GameState::new();

        // Add 2 cards to library to draw
        for i in 0..2 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: format!("Forest {}", i),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.library.add_card(land);
        }

        // Add 3 lands to hand (excess)
        for i in 0..3 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: format!("Island {}", i),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Blue],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.hand.add_card(land);
        }

        resolve_kiora_etb(&mut state, false);

        // Should have discarded 2 cards (2 lands)
        assert_eq!(state.graveyard.size(), 2);
        // Should have 3 lands in hand still (drew 2, discarded 2)
        let lands_in_hand = state.hand.cards().iter().filter(|c| matches!(c, Card::Land(_))).count();
        assert_eq!(lands_in_hand, 3);
    }

    #[test]
    fn test_resolve_kiora_etb_discards_last_card_when_no_priority() {
        let mut state = GameState::new();

        // Add 2 cards to library to draw
        for i in 0..2 {
            let land = Card::Land(LandCard {
                base: BaseCard {
                    name: format!("Forest {}", i),
                    mana_cost: ManaCost::default(),
                    mana_value: 0,
                },
                subtype: LandSubtype::Basic,
                enters_tapped: false,
                colors: vec![ManaColor::Green],
                has_surveil: false,
                surveil_amount: 0,
            });
            state.library.add_card(land);
        }

        // Add 1 creature to hand (not Bringer or Terror)
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Town Greeter".to_string(),
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
        state.hand.add_card(creature);

        resolve_kiora_etb(&mut state, false);

        // Should have discarded 2 cards
        assert_eq!(state.graveyard.size(), 2);
        // Hand should have 1 card (1 creature + 2 drawn - 2 discarded)
        assert_eq!(state.hand.size(), 1);
    }

    #[test]
    fn test_resolve_kiora_etb_with_empty_library() {
        let mut state = GameState::new();

        // Add 2 cards to hand
        for i in 0..2 {
            let creature = Card::Creature(CreatureCard {
                base: BaseCard {
                    name: format!("Creature {}", i),
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
            state.hand.add_card(creature);
        }

        // Library is empty
        resolve_kiora_etb(&mut state, false);

        // Should have tried to draw but couldn't (library empty)
        // Should have discarded 2 cards (the creatures)
        assert_eq!(state.hand.size(), 0); // 2 - 2 discarded
        assert_eq!(state.graveyard.size(), 2); // 2 discarded
    }
}

