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

