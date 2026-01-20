use crate::card::{Card, CardDatabase};
use crate::game::state::GameState;
use crate::game::turns::{start_turn, draw_phase, upkeep_phase, end_phase};
use crate::game::cards;
use crate::simulation::decisions::DecisionEngine;
use crate::rng::GameRng;
use crate::simulation::mulligan::resolve_mulligans;

/// Result of a single game simulation
#[derive(Debug, Clone)]
pub struct GameResult {
    /// Turn on which the game was won (None if didn't win by turn 20)
    pub win_turn: Option<u32>,
    /// Whether we were on the play (true) or draw (false)
    pub on_the_play: bool,
    /// Total damage dealt via combat
    pub total_combat_damage: u32,
    /// Total damage dealt via non-combat sources (combo)
    pub combo_damage: u32,
    /// First turn we had access to U mana
    pub turn_with_u: Option<u32>,
    /// First turn we had access to B mana
    pub turn_with_b: Option<u32>,
    /// First turn we had access to G mana
    pub turn_with_g: Option<u32>,
    /// First turn we had access to U, B, and G mana
    pub turn_with_ubg: Option<u32>,
}

/// Check if the game has been won
pub fn check_win_condition(state: &GameState) -> bool {
    state.opponent_life <= 0
}

/// Get available mana colors from battlefield lands
fn get_available_colors(state: &GameState) -> std::collections::HashSet<String> {
    let mut colors = std::collections::HashSet::new();

    for permanent in state.battlefield.permanents() {
        match &permanent.card {
            Card::Land(land) => {
                // Add colors this land can produce
                for color in &land.colors {
                    let color_str = match color {
                        crate::card::ManaColor::White => "W".to_string(),
                        crate::card::ManaColor::Blue => "U".to_string(),
                        crate::card::ManaColor::Black => "B".to_string(),
                        crate::card::ManaColor::Red => "R".to_string(),
                        crate::card::ManaColor::Green => "G".to_string(),
                        crate::card::ManaColor::Colorless => "C".to_string(),
                    };
                    colors.insert(color_str);
                }
            }
            _ => {}
        }
    }

    colors
}

/// Simulate combat phase: declare attackers and deal damage
pub fn simulate_combat(state: &mut GameState, verbose: bool) -> u32 {
    let mut total_damage = 0;

    // Find eligible attackers (creatures without summoning sickness, not tapped)
    let mut attackers = Vec::new();
    for (idx, permanent) in state.battlefield.permanents().iter().enumerate() {
        // Must be a creature
        if !matches!(permanent.card, Card::Creature(_)) {
            continue;
        }

        // Check summoning sickness (entered before this turn)
        if permanent.turn_entered >= state.turn {
            continue;
        }

        // Check if tapped
        if permanent.tapped {
            continue;
        }

        attackers.push(idx);
    }

    // Tap all attackers and calculate damage
    for idx in attackers {
        if let Some(permanent) = state.battlefield.permanents_mut().get_mut(idx) {
            permanent.tapped = true;

            // Get creature power
            if let Card::Creature(creature) = &permanent.card {
                total_damage += creature.power as u32;
            }
        }
    }

    // Deal damage to opponent
    state.opponent_life -= total_damage as i32;

    if verbose && total_damage > 0 {
        println!("[Combat] {} damage dealt", total_damage);
    }

    total_damage
}

/// Execute a single turn: untap -> draw -> main -> combat -> end
pub fn execute_turn(state: &mut GameState, db: &CardDatabase, verbose: bool) -> u32 {
    // Start turn: increment turn counter, untap, reset land drop
    start_turn(state);

    if verbose {
        println!("\n=== TURN {} ===", state.turn);
    }

    // Upkeep phase
    upkeep_phase(state);

    // Draw phase
    state.phase = crate::game::state::Phase::Draw;
    let hand_before = state.hand.size();
    draw_phase(state);

    if verbose {
        if state.hand.size() > hand_before {
            // Get the last card drawn
            if let Some(card) = state.hand.cards().last() {
                println!("[Draw] Drew: {}", card.name());
            }
        } else if state.turn == 1 && state.on_the_play {
            println!("[Draw] Skipped (on the play)");
        }
    }

    // Main phase 1: Play lands and cast spells
    state.phase = crate::game::state::Phase::Main1;
    if verbose {
        let hand_names: Vec<&str> = state.hand.cards().iter().map(|c| c.name()).collect();
        println!("[Main 1] Hand: {}", hand_names.join(", "));
    }
    execute_main_phase(state, db, verbose);

    // Combat phase
    state.phase = crate::game::state::Phase::Combat;
    let combat_damage = simulate_combat(state, verbose);

    // Main phase 2: Additional spell casting could happen here
    state.phase = crate::game::state::Phase::Main2;
    // For now, we don't do anything in main 2

    // End phase
    state.phase = crate::game::state::Phase::End;
    end_phase(state);

    if verbose {
        println!("[End of Turn {}]", state.turn);
        let battlefield_names: Vec<String> = state.battlefield.permanents()
            .iter()
            .map(|p| {
                let mut name = p.card.name().to_string();
                if let Some(copy_of) = &p.is_copy_of {
                    name.push_str(&format!(" (copy of {})", copy_of));
                }
                if let Some(time_counters) = p.counters.get(&crate::game::zones::CounterType::Time) {
                    name.push_str(&format!(" ({} time counters)", time_counters));
                }
                name
            })
            .collect();
        println!("  Battlefield: {}", if battlefield_names.is_empty() { "(empty)".to_string() } else { battlefield_names.join(", ") });

        let graveyard_names: Vec<&str> = state.graveyard.cards().iter().map(|c| c.name()).collect();
        println!("  Graveyard: {}", if graveyard_names.is_empty() { "(empty)".to_string() } else { graveyard_names.join(", ") });

        println!("  Opponent life: {}", state.opponent_life);
    }

    combat_damage
}

/// Get mana cost from a card
fn get_mana_cost(card: &Card) -> &crate::card::ManaCost {
    match card {
        Card::Land(c) => &c.base.mana_cost,
        Card::Creature(c) => &c.base.mana_cost,
        Card::Instant(c) => &c.base.mana_cost,
        Card::Sorcery(c) => &c.base.mana_cost,
        Card::Enchantment(c) => &c.base.mana_cost,
        Card::Saga(c) => &c.base.mana_cost,
    }
}

/// Execute main phase: play lands and cast spells
fn execute_main_phase(state: &mut GameState, db: &CardDatabase, verbose: bool) {
    // Step 1: Play one land if we haven't already (play BEFORE tapping for mana!)
    if !state.land_played_this_turn {
        let hand_cards = state.hand.cards().to_vec();
        if let Some(land_idx) = DecisionEngine::choose_land_to_play(&hand_cards, state) {
            if let Some(card) = state.hand.remove_card(land_idx) {
                let card_name = card.name().to_string();
                let _ = cards::play_land(state, &card);
                if verbose {
                    // Check if land entered tapped
                    let last_perm = state.battlefield.permanents().last();
                    let tapped_str = if let Some(perm) = last_perm {
                        if perm.tapped { " (tapped)" } else { "" }
                    } else {
                        ""
                    };
                    println!("  [Land] {}{}", card_name, tapped_str);
                }
            }
        }
    }

    // Step 2: Tap all untapped lands for mana (including newly played land)
    let mut lands_to_tap = Vec::new();
    for (idx, permanent) in state.battlefield.permanents().iter().enumerate() {
        if matches!(permanent.card, Card::Land(_)) && !permanent.tapped {
            lands_to_tap.push(idx);
        }
    }

    // Tap lands and add mana to pool
    for idx in lands_to_tap {
        if let Some(permanent) = state.battlefield.permanents_mut().get_mut(idx) {
            let _ = cards::tap_land_for_mana(permanent, &mut state.mana_pool);
        }
    }

    // Step 3: Check if combo is ready (Spider-Man + Bringer in graveyard + 4+ mana)
    if DecisionEngine::is_combo_ready(state) {
        // Cast Spider-Man to trigger the combo
        let hand_cards = state.hand.cards().to_vec();
        if let Some(spider_idx) = hand_cards.iter().position(|c| c.name() == "Superior Spider-Man") {
            if let Some(card) = state.hand.remove_card(spider_idx) {
                // Pay mana for Spider-Man
                let cost = get_mana_cost(&card);
                if state.mana_pool.pay(cost) {
                    // Cast as creature
                    let _ = cards::cast_creature(state, &card, false);

                    // Process ETB triggers (this will copy Bringer and trigger mass reanimate)
                    // We need to get the permanent index first to avoid double borrow
                    let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                    if perm_idx < state.battlefield.permanents().len() {
                        let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                        let _ = cards::process_etb_triggers(state, &mut perm, db);
                        state.battlefield.permanents_mut()[perm_idx] = perm;
                    }
                }
            }
        }
        return; // Stop casting after combo
    }

    // Step 4: Cast spells from hand until we run out of mana or cards
    loop {
        let hand_cards = state.hand.cards().to_vec();

        // Find the best card to play
        if let Some(card_idx) = DecisionEngine::choose_card_to_play(&hand_cards, state, db) {
            let card = hand_cards[card_idx].clone();

            // Check if we can cast it
            if !cards::can_cast(&card, &state.mana_pool) {
                break; // Can't cast anything else
            }

            // Remove from hand
            let _ = state.hand.remove_card(card_idx);

            // Pay mana
            let cost = get_mana_cost(&card);
            if !state.mana_pool.pay(cost) {
                // Put it back if we can't pay
                state.hand.add_card(card);
                break;
            }

            // Cast the card based on type
            let card_name = card.name().to_string();
            match &card {
                Card::Creature(_) => {
                    let _ = cards::cast_creature(state, &card, false);

                    // Process ETB triggers
                    let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                    if perm_idx < state.battlefield.permanents().len() {
                        let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                        let _ = cards::process_etb_triggers(state, &mut perm, db);
                        state.battlefield.permanents_mut()[perm_idx] = perm;
                    }

                    if verbose {
                        println!("  [Cast] {}", card_name);
                    }
                }
                Card::Land(_) => {
                    let _ = cards::play_land(state, &card);
                    if verbose {
                        println!("  [Land] {}", card_name);
                    }
                }
                Card::Instant(_) | Card::Sorcery(_) | Card::Enchantment(_) => {
                    let _ = cards::cast_spell(state, &card, db);
                    if verbose {
                        println!("  [Cast] {}", card_name);
                    }
                }
                Card::Saga(_) => {
                    let _ = cards::cast_spell(state, &card, db);
                    if verbose {
                        println!("  [Cast] {}", card_name);
                    }
                }
            }
        } else {
            break; // No more cards to play
        }
    }
}

/// Run a complete game simulation
pub fn run_game(
    deck: &[Card],
    seed: u64,
    _db: &CardDatabase,
    verbose: bool,
) -> GameResult {
    let mut rng = GameRng::new(Some(seed));

    // Initialize game state
    let mut state = GameState::new();

    // Determine if on play or draw (50/50) - BEFORE shuffling to match TypeScript RNG sequence
    state.on_the_play = rng.random() < 0.5;

    // Shuffle deck into library
    let mut shuffled_deck = deck.to_vec();
    rng.shuffle(&mut shuffled_deck);
    for card in shuffled_deck {
        state.library.add_card(card);
    }

    // Mulligan phase: resolve mulligans to get opening hand
    let mut library_cards = Vec::new();
    for _ in 0..state.library.size() {
        if let Some(card) = state.library.draw() {
            library_cards.push(card);
        }
    }

    let opening_hand = resolve_mulligans(&mut library_cards, &mut rng);

    // Put remaining cards back in library
    for card in library_cards {
        state.library.add_card(card);
    }

    // Add opening hand to hand
    for card in opening_hand.clone() {
        state.hand.add_card(card);
    }

    // Print game start info if verbose
    if verbose {
        println!("=== Game Start (seed: {}) ===", seed);
        println!("{}", if state.on_the_play { "On the play" } else { "On the draw" });
        println!("Opening hand ({} cards):", opening_hand.len());
        for card in &opening_hand {
            println!("  - {}", card.name());
        }
    }
    
    // Game loop
    let mut total_combat_damage = 0u32;
    let mut combo_damage = 0u32;
    let max_turns = 20u32;
    
    let mut turn_with_u = None;
    let mut turn_with_b = None;
    let mut turn_with_g = None;
    let mut turn_with_ubg = None;
    
    while state.turn < max_turns && !check_win_condition(&state) {
        let life_before = state.opponent_life;

        // Execute turn
        let combat_damage = execute_turn(&mut state, _db, verbose);
        total_combat_damage += combat_damage;
        
        // Track combo damage (non-combat damage)
        let total_damage_this_turn = (life_before - state.opponent_life) as u32;
        combo_damage += total_damage_this_turn.saturating_sub(combat_damage);
        
        // Track when colors become available
        let colors = get_available_colors(&state);
        if turn_with_u.is_none() && colors.contains("U") {
            turn_with_u = Some(state.turn);
        }
        if turn_with_b.is_none() && colors.contains("B") {
            turn_with_b = Some(state.turn);
        }
        if turn_with_g.is_none() && colors.contains("G") {
            turn_with_g = Some(state.turn);
        }
        if turn_with_ubg.is_none() && colors.contains("U") && colors.contains("B") && colors.contains("G") {
            turn_with_ubg = Some(state.turn);
        }
    }
    
    GameResult {
        win_turn: if check_win_condition(&state) { Some(state.turn) } else { None },
        on_the_play: state.on_the_play,
        total_combat_damage,
        combo_damage,
        turn_with_u,
        turn_with_b,
        turn_with_g,
        turn_with_ubg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{BaseCard, CreatureCard, LandCard};

    #[test]
    fn test_check_win_condition_false() {
        let state = GameState::new();
        assert!(!check_win_condition(&state));
    }

    #[test]
    fn test_check_win_condition_true() {
        let mut state = GameState::new();
        state.opponent_life = 0;
        assert!(check_win_condition(&state));
    }

    #[test]
    fn test_check_win_condition_negative() {
        let mut state = GameState::new();
        state.opponent_life = -5;
        assert!(check_win_condition(&state));
    }

    #[test]
    fn test_get_available_colors_empty() {
        let state = GameState::new();
        let colors = get_available_colors(&state);
        assert!(colors.is_empty());
    }

    #[test]
    fn test_get_available_colors_with_lands() {
        use crate::card::{LandSubtype, ManaColor};

        let mut state = GameState::new();

        // Add a forest (green land)
        let forest = Card::Land(LandCard {
            base: BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: LandSubtype::Basic,
            colors: vec![ManaColor::Green],
            enters_tapped: false,
            has_surveil: false,
            surveil_amount: 0,
        });

        let permanent = crate::game::zones::Permanent::new(forest, 1);
        state.battlefield.add_permanent(permanent);

        let colors = get_available_colors(&state);
        assert!(colors.contains("G"));
        assert_eq!(colors.len(), 1);
    }

    #[test]
    fn test_simulate_combat_no_creatures() {
        let mut state = GameState::new();
        let damage = simulate_combat(&mut state, false);
        assert_eq!(damage, 0);
        assert_eq!(state.opponent_life, 20);
    }

    #[test]
    fn test_simulate_combat_with_creature() {
        let mut state = GameState::new();
        state.turn = 2; // Avoid summoning sickness
        
        // Add a creature to battlefield
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: Default::default(),
                mana_value: 1,
            },
            power: 3,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        
        let permanent = crate::game::zones::Permanent::new(creature, 1);
        state.battlefield.add_permanent(permanent);

        let damage = simulate_combat(&mut state, false);
        assert_eq!(damage, 3);
        assert_eq!(state.opponent_life, 17);
    }

    #[test]
    fn test_simulate_combat_summoning_sickness() {
        let mut state = GameState::new();
        state.turn = 1;
        
        // Add a creature that entered this turn (has summoning sickness)
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: Default::default(),
                mana_value: 1,
            },
            power: 3,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        
        let permanent = crate::game::zones::Permanent::new(creature, 1);
        state.battlefield.add_permanent(permanent);
        
        let damage = simulate_combat(&mut state, false);
        assert_eq!(damage, 0); // Can't attack due to summoning sickness
        assert_eq!(state.opponent_life, 20);
    }
}

