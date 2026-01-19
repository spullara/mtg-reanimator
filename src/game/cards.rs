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
        Card::Instant(spell) | Card::Sorcery(spell) | Card::Enchantment(spell) => {
            // Process spell abilities
            for ability in &spell.abilities {
                match ability.as_str() {
                    "mill_4_return_permanent" => {
                        // Cache Grab: mill 4, return permanent
                        let milled = state.library.mill(4);
                        for card in milled {
                            state.graveyard.add_card(card);
                        }
                    }
                    "etb_mill_4_return_artifact_creature_land" => {
                        // Dredger's Insight: mill 4, return artifact/creature/land
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
        Card::Saga(saga) => {
            // Sagas enter with 1 lore counter
            state.saga_counters.insert(saga.base.name.clone(), 1);
            Ok(())
        }
        _ => Err("Not a spell card".to_string()),
    }
}

/// Advance saga to next chapter
pub fn advance_saga(state: &mut GameState, saga_name: &str) -> Result<(), String> {
    let current_counters = state.saga_counters.get(saga_name).copied().unwrap_or(0);
    let new_counters = current_counters + 1;
    state.saga_counters.insert(saga_name.to_string(), new_counters);
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
            _ => {} // Other abilities handled elsewhere
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{BaseCard, CreatureCard, ManaCost};

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
}

