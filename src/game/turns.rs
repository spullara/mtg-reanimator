use crate::card::Card;
use crate::game::state::GameState;
use crate::game::zones::CounterType;

/// Start a new turn: increment turn counter, untap all permanents, reset land drop
pub fn start_turn(state: &mut GameState) {
    state.turn += 1;
    state.reset_turn_state();
    state.untap_all();
}

/// Draw phase: draw 1 card (skip on turn 1 if on play), advance saga counters
pub fn draw_phase(state: &mut GameState) {
    // Skip draw on turn 1 if on the play
    if state.turn == 1 && state.on_the_play {
        return;
    }

    // Draw a card
    state.draw_card();

    // Advance saga counters and resolve chapters
    for permanent in state.battlefield.permanents_mut() {
        if matches!(permanent.card, Card::Saga(_)) {
            // Only advance if saga was cast before this turn
            if permanent.turn_entered < state.turn {
                permanent.add_counter(CounterType::Time, 1);
                // Note: Chapter resolution would happen here in full implementation
            }
        }
    }
}

/// Upkeep phase: trigger upkeep effects (saga counter advancement if needed)
pub fn upkeep_phase(_state: &mut GameState) {
    // Upkeep effects would be triggered here
    // For now, saga advancement happens in draw_phase
}

/// End phase: decrement time counters (impending), discard to 7
pub fn end_phase(state: &mut GameState) {
    // Decrement time counters on impending permanents
    for permanent in state.battlefield.permanents_mut() {
        let time_counters = permanent.get_counter(CounterType::Time);
        if time_counters > 0 {
            permanent.remove_counter(CounterType::Time, 1);
        }
    }

    // Discard to hand size 7 if needed
    while state.hand.size() > 7 {
        // In a full implementation, this would choose which card to discard
        // For now, just remove the last card
        if let Some(card) = state.hand.remove_card(state.hand.size() - 1) {
            state.add_to_graveyard(card);
        }
    }
}

/// Check if a creature can attack (not affected by summoning sickness)
pub fn can_attack(state: &GameState, permanent_index: usize) -> bool {
    if let Some(permanent) = state.battlefield.permanents().get(permanent_index) {
        // Creature can attack if it entered before this turn
        permanent.turn_entered < state.turn
    } else {
        false
    }
}

/// Check if player can play a land this turn
pub fn can_play_land(state: &GameState) -> bool {
    !state.land_played_this_turn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{BaseCard, CreatureCard};

    #[test]
    fn test_start_turn_increments_turn() {
        let mut state = GameState::new();
        assert_eq!(state.turn, 0);
        start_turn(&mut state);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn test_start_turn_resets_land_played() {
        let mut state = GameState::new();
        state.land_played_this_turn = true;
        start_turn(&mut state);
        assert_eq!(state.land_played_this_turn, false);
    }

    #[test]
    fn test_draw_phase_skips_turn_1_on_play() {
        let mut state = GameState::new();
        state.on_the_play = true;
        state.turn = 1;
        let hand_size_before = state.hand.size();
        draw_phase(&mut state);
        assert_eq!(state.hand.size(), hand_size_before);
    }

    #[test]
    fn test_draw_phase_draws_on_turn_2() {
        let mut state = GameState::new();
        state.on_the_play = true;
        state.turn = 2;
        let card = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test".to_string(),
                mana_cost: Default::default(),
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
        state.library.add_card(card);
        draw_phase(&mut state);
        assert_eq!(state.hand.size(), 1);
    }

    #[test]
    fn test_can_attack_summoning_sickness() {
        let mut state = GameState::new();
        state.turn = 1;
        let card = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test".to_string(),
                mana_cost: Default::default(),
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
        let permanent = crate::game::zones::Permanent::new(card, 1);
        state.battlefield.add_permanent(permanent);
        
        // Can't attack on turn 1 (entered this turn)
        assert!(!can_attack(&state, 0));
        
        // Can attack on turn 2
        state.turn = 2;
        assert!(can_attack(&state, 0));
    }

    #[test]
    fn test_can_play_land() {
        let mut state = GameState::new();
        assert!(can_play_land(&state));
        state.land_played_this_turn = true;
        assert!(!can_play_land(&state));
    }
}

