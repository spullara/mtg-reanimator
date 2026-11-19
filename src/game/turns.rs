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

/// Draw phase: draw 1 card (skip on turn 1 if on play)
pub fn draw_phase(state: &mut GameState) {
    // Skip draw on turn 1 if on the play
    if state.turn == 1 && state.on_the_play {
        return;
    }

    // Draw a card
    state.draw_card();
}

/// Precombat main phase start: advance saga counters and resolve chapters
/// According to MTG rules, saga lore counters are added at the beginning of the precombat main phase
pub fn precombat_main_phase_start(state: &mut GameState, verbose: bool) {
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

    // Fourth pass: remove completed sagas (put in graveyard)
    let mut indices_to_remove: Vec<usize> = Vec::new();
    for (idx, _name, max_chapters) in &sagas_to_advance {
        let permanent = &state.battlefield.permanents()[*idx];
        let counters = permanent.get_counter(CounterType::Time) as usize;
        if counters >= *max_chapters {
            indices_to_remove.push(*idx);
        }
    }

    // Remove in reverse order to preserve indices, putting sagas in graveyard
    for idx in indices_to_remove.into_iter().rev() {
        if let Some(permanent) = state.battlefield.remove_permanent(idx) {
            state.graveyard.add_card(permanent.card);
        }
    }
}

/// Upkeep phase: trigger upkeep effects
pub fn upkeep_phase(_state: &mut GameState) {
    // Upkeep effects would be triggered here
    // Saga advancement happens in precombat_main_phase_start (per MTG rules)
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

    // Discard to hand size 7, choosing like the other discard effects:
    // a reanimation target first, then a blank, and keep Spider-Man
    while state.hand.size() > 7 {
        let idx = crate::simulation::decisions::DecisionEngine::choose_discard(state)
            .unwrap_or(state.hand.size() - 1);
        if let Some(card) = state.hand.remove_card(idx) {
            state.add_to_graveyard(card);
        }
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::card::CardDatabase;

    #[test]
    fn test_cleanup_discards_like_other_discard_effects() {
        let db = CardDatabase::from_file("cards.json").expect("cards");
        let mut state = GameState::new();
        // 8 cards: Spider-Man last, a Bringer in the middle
        for n in ["Watery Grave", "Swamp", "Bringer of the Last Gift", "Does Machines",
                  "Watery Grave", "Swamp", "Watery Grave", "Superior Spider-Man"] {
            state.hand.add_card(db.get_card(n).expect(n));
        }
        end_phase(&mut state);
        assert_eq!(state.hand.size(), 7);
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift"),
            "the reanimation target is discarded");
        assert!(state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man"),
            "Spider-Man is kept");
    }
}
