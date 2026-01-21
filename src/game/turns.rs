use crate::card::Card;
use crate::game::state::GameState;
use crate::game::zones::CounterType;
use crate::game::cards;

/// Start a new turn: increment turn counter, untap all permanents, reset land drop
pub fn start_turn(state: &mut GameState) {
    state.turn += 1;
    state.reset_turn_state();
    state.untap_all();
}

/// Draw phase: draw 1 card (skip on turn 1 if on play), advance saga counters
pub fn draw_phase(state: &mut GameState, verbose: bool) {
    // Skip draw on turn 1 if on the play
    if state.turn == 1 && state.on_the_play {
        return;
    }

    // Draw a card
    state.draw_card();

    // First pass: collect saga info (names, turn_entered) without modifying
    let mut sagas_to_advance: Vec<(usize, String, usize)> = Vec::new(); // (index, name, max_chapters)
    
    for (i, permanent) in state.battlefield.permanents().iter().enumerate() {
        if let Card::Saga(saga) = &permanent.card {
            // Only advance if saga was cast before this turn
            if permanent.turn_entered < state.turn {
                sagas_to_advance.push((i, saga.base.name.clone(), saga.chapters.len()));
            }
        }
    }
    
    // Second pass: advance counters and collect chapters to resolve
    let mut saga_chapters: Vec<(String, usize)> = Vec::new();
    
    for (idx, name, _max_chapters) in &sagas_to_advance {
        let permanent = &mut state.battlefield.permanents_mut()[*idx];
        permanent.add_counter(CounterType::Time, 1);
        let chapter = permanent.get_counter(CounterType::Time) as usize;
        saga_chapters.push((name.clone(), chapter));
    }
    
    // Third pass: resolve chapters
    for (saga_name, chapter) in &saga_chapters {
        cards::resolve_saga_chapter(state, saga_name, *chapter as u32, verbose);
    }
    
    // Fourth pass: remove completed sagas
    let mut indices_to_remove: Vec<usize> = Vec::new();
    for (idx, _name, max_chapters) in &sagas_to_advance {
        let permanent = &state.battlefield.permanents()[*idx];
        let counters = permanent.get_counter(CounterType::Time) as usize;
        if counters >= *max_chapters {
            indices_to_remove.push(*idx);
        }
    }
    
    // Remove in reverse order to preserve indices
    for idx in indices_to_remove.into_iter().rev() {
        state.battlefield.remove_permanent(idx);
    }
}

/// Upkeep phase: trigger upkeep effects (saga counter advancement if needed)
pub fn upkeep_phase(_state: &mut GameState) {
    // Upkeep effects would be triggered here
    // For now, saga advancement happens in draw_phase
}

/// End phase: decrement time counters (impending creatures only, NOT sagas), discard to 7
pub fn end_phase(state: &mut GameState) {
    // Decrement time counters on impending creatures only
    // Sagas also use time counters but they count UP, not down - don't touch them!
    for permanent in state.battlefield.permanents_mut() {
        // Only decrement counters on creatures (impending creatures use time counters)
        // Sagas are NOT creatures - they use Card::Saga variant
        if matches!(permanent.card, Card::Creature(_)) {
            let time_counters = permanent.get_counter(CounterType::Time);
            if time_counters > 0 {
                permanent.remove_counter(CounterType::Time, 1);
            }
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
