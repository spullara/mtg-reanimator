use crate::card::{Card, CardDatabase, LandCard, LandSubtype, ManaColor};
use crate::game::state::GameState;
use crate::game::turns::{start_turn, draw_phase, upkeep_phase, end_phase};
use crate::game::cards;
use crate::game::mana;
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

/// Port of TypeScript mainPhase function (lines 2211-2502)
/// Core game logic that determines what spells to cast and in what order
pub fn main_phase(state: &mut GameState, db: &CardDatabase, verbose: bool) {
    // SPECIAL CASE: Turn 4 combo check
    // If we have Spider-Man in hand, Bringer in GY, and can get to 4 mana by playing a land,
    // play the land FIRST before casting any other spells!
    let has_spider_man = state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man");
    let has_bringer_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
    let current_mana = state.battlefield.permanents()
        .iter()
        .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
        .count() as u32;

    if has_spider_man && has_bringer_in_gy && current_mana == 3 && !state.land_played_this_turn {
        // Check if we have an untapped land to play
        let hand_cards = state.hand.cards().to_vec();
        if let Some(untapped_land_idx) = hand_cards.iter().position(|c| {
            if let Card::Land(land) = c {
                // Check if land enters untapped
                !land.enters_tapped && land.subtype != crate::card::LandSubtype::Fastland
            } else {
                false
            }
        }) {
            if let Some(untapped_land) = state.hand.remove_card(untapped_land_idx) {
                let land_name = untapped_land.name().to_string();
                let _ = cards::play_land(state, &untapped_land);
                if verbose {
                    println!("  [COMBO SETUP] Played {} first to enable turn 4 combo", land_name);
                }
            }
        }
    }

    // STEP 1: If we haven't played a land yet and have land-finding spells,
    // cast those FIRST to potentially find a better land
    // BUT: If we have Bringer/Terror in hand and can cast Kiora, skip this step!
    // Kiora is more important (discards Bringer to graveyard for the combo)
    let has_bringer_or_terror_in_hand = state.hand.cards().iter().any(|c| {
        c.name() == "Bringer of the Last Gift" || c.name() == "Terror of the Peaks"
    });
    let kiora_in_hand = state.hand.cards().iter().find(|c| c.name() == "Kiora, the Rising Tide");

    // Check if we can cast Kiora now OR if we could cast it after playing an untapped land
    let could_cast_kiora_after_land_drop = || -> bool {
        let kiora = match kiora_in_hand {
            Some(k) => k,
            None => return false,
        };

        // Can cast now?
        if mana::can_cast_spell(kiora, state) {
            return true;
        }

        // If we've already played a land, no look-ahead needed
        if state.land_played_this_turn {
            return false;
        }

        // Check if playing an untapped land would enable Kiora
        let current_mana = state.battlefield.permanents().iter()
            .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
            .count() as u32;
        let kiora_cost = kiora.mana_value();

        // Would one more mana be enough?
        if current_mana + 1 < kiora_cost {
            return false;
        }

        // Helper: check if land enters tapped
        let land_enters_tapped = |land: &LandCard| -> bool {
            match land.subtype {
                LandSubtype::Fastland => {
                    let land_count = state.battlefield.permanents().iter()
                        .filter(|p| matches!(p.card, Card::Land(_)))
                        .count();
                    land_count >= 3
                }
                LandSubtype::Town => state.turn > 3,
                LandSubtype::Shock => state.life <= 2, // Enters tapped if we can't pay 2 life
                _ => land.enters_tapped,
            }
        };

        // Check if we have an untapped land that produces U (Kiora needs U)
        let has_untapped_land_with_u = state.hand.cards().iter().any(|c| {
            if let Card::Land(land) = c {
                if land_enters_tapped(land) {
                    return false;
                }
                // Check if land produces U
                land.colors.contains(&ManaColor::Blue)
            } else {
                false
            }
        });

        // Also check if we already have U available and just need an untapped land for mana count
        let has_u_available = state.battlefield.permanents().iter().any(|p| {
            if p.tapped {
                return false;
            }
            if let Card::Land(land) = &p.card {
                land.colors.contains(&ManaColor::Blue)
            } else {
                false
            }
        });

        if has_u_available {
            // We have U, just need any untapped land for the mana count
            let has_any_untapped_land = state.hand.cards().iter().any(|c| {
                if let Card::Land(land) = c {
                    !land_enters_tapped(land)
                } else {
                    false
                }
            });
            return has_any_untapped_land;
        }

        // We need the new land to provide U
        has_untapped_land_with_u
    };

    let should_prioritize_kiora = has_bringer_or_terror_in_hand
        && kiora_in_hand.is_some()
        && could_cast_kiora_after_land_drop();

    if !state.land_played_this_turn && !should_prioritize_kiora {
        let mut cast_any = true;

        while cast_any && !state.land_played_this_turn {
            cast_any = false;

            // Land-finding spells (from TypeScript LAND_FINDING_SPELLS)
            let land_finders = vec![
                "Cache Grab",
                "Dredger's Insight",
                "Town Greeter",
            ];

            // Find castable land-finding spells
            let mut castable_finders: Vec<(usize, &Card)> = state.hand.cards()
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    land_finders.contains(&c.name()) && mana::can_cast_spell(c, state)
                })
                .collect();

            if !castable_finders.is_empty() {
                // Sort by mana value (cheaper first)
                castable_finders.sort_by_key(|(_, c)| c.mana_value());

                let (spell_idx, spell) = castable_finders[0];
                let lands_before = state.hand.cards().iter().filter(|c| matches!(c, Card::Land(_))).count();

                // Remove from hand and cast
                if let Some(card) = state.hand.remove_card(spell_idx) {
                    let cost = get_mana_cost(&card);
                    if mana::tap_lands_for_cost(cost, state, None) {
                        let card_name = card.name().to_string();

                        // Handle creatures specially (add to battlefield and process ETB)
                        if matches!(&card, Card::Creature(_)) {
                            let _ = cards::cast_creature(state, &card, false);

                            // Process ETB triggers
                            let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                            if perm_idx < state.battlefield.permanents().len() {
                                let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                                let _ = cards::process_etb_triggers_verbose(state, &mut perm, db, verbose);
                                state.battlefield.permanents_mut()[perm_idx] = perm;
                            }
                        } else {
                            let _ = cards::cast_spell(state, &card, db);
                        }

                        if verbose {
                            println!("  [Cast] {}", card_name);
                        }
                        cast_any = true;

                        // Check if we found a land
                        let lands_after = state.hand.cards().iter().filter(|c| matches!(c, Card::Land(_))).count();
                        if lands_after > lands_before && verbose {
                            println!("  [Land-finder] Found a land");
                        }
                    } else {
                        // Put it back if we can't pay
                        state.hand.add_card(card);
                    }
                }
            } else {
                break; // No more land-finding spells
            }
        }
    }

    // STEP 2: Now play a land (possibly one we just found from milling)
    if !state.land_played_this_turn {
        let hand_cards = state.hand.cards().to_vec();
        let lands_in_hand: Vec<&Card> = hand_cards.iter()
            .filter(|c| matches!(c, Card::Land(_)))
            .collect();

        if !lands_in_hand.is_empty() {
            // Use DecisionEngine to choose the best land
            if let Some(land_idx) = DecisionEngine::choose_land_to_play(&hand_cards, state) {
                if let Some(card) = state.hand.remove_card(land_idx) {
                    let card_name = card.name().to_string();
                    let _ = cards::play_land(state, &card);

                    // DO NOT tap the land here - TypeScript taps lands DURING casting
                    // This allows can_cast_spell to correctly see the new untapped land

                    if verbose {
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
    }

    // STEP 3: Cast remaining spells
    let mut cast_any = true;
    while cast_any {
        cast_any = false;

        // Get game state for spell priorities
        let has_bringer_in_graveyard = state.graveyard.cards().iter()
            .any(|c| c.name() == "Bringer of the Last Gift");
        let has_bringer_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Bringer of the Last Gift");
        let has_terror_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Terror of the Peaks");

        // Check if the combo would be lethal
        let combo_is_lethal = has_bringer_in_graveyard && cards::is_combo_lethal(state);
        let has_spider_man_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Superior Spider-Man");

        // Log when we're holding back the combo
        if verbose && has_bringer_in_graveyard && has_spider_man_in_hand && !combo_is_lethal {
            let expected_damage = cards::calculate_combo_damage(state);
            println!(
                "  [Waiting] Combo not lethal yet (expected: {} damage, need: {})",
                expected_damage, state.opponent_life
            );
        }

        // Get castable spells
        let mut castable_spells: Vec<(usize, &Card)> = state.hand.cards()
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                if matches!(c, Card::Land(_)) {
                    return false;
                }
                if !mana::can_cast_spell(c, state) {
                    return false;
                }

                // Only cast Spider-Man if the combo would be LETHAL
                if c.name() == "Superior Spider-Man" {
                    if !has_bringer_in_graveyard {
                        return false; // Need Bringer in graveyard
                    }
                    if !combo_is_lethal {
                        return false; // Wait until it would kill
                    }
                }

                true
            })
            .collect();

        if castable_spells.is_empty() {
            break;
        }

        // Sort by priority
        castable_spells.sort_by(|a, b| {
            let (_, a_card) = a;
            let (_, b_card) = b;

            // Priority 1: Spider-Man if combo is lethal
            if combo_is_lethal {
                if a_card.name() == "Superior Spider-Man" {
                    return std::cmp::Ordering::Less;
                }
                if b_card.name() == "Superior Spider-Man" {
                    return std::cmp::Ordering::Greater;
                }
            }

            // Priority 2: Kiora if Bringer/Terror in hand
            if has_bringer_in_hand {
                if a_card.name() == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Less;
                }
                if b_card.name() == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Greater;
                }
            }

            if has_terror_in_hand {
                if a_card.name() == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Less;
                }
                if b_card.name() == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Greater;
                }
            }

            // Priority 3: Mill spells
            let mill_spells = vec![
                "Cache Grab",
                "Dredger's Insight",
                "Town Greeter",
                "Overlord of the Balemurk",
            ];
            let a_is_mill = mill_spells.contains(&a_card.name());
            let b_is_mill = mill_spells.contains(&b_card.name());
            if a_is_mill && !b_is_mill {
                return std::cmp::Ordering::Less;
            }
            if b_is_mill && !a_is_mill {
                return std::cmp::Ordering::Greater;
            }

            // Priority 4: Awaken the Honored Dead
            if a_card.name() == "Awaken the Honored Dead" && !b_is_mill {
                return std::cmp::Ordering::Less;
            }
            if b_card.name() == "Awaken the Honored Dead" && !a_is_mill {
                return std::cmp::Ordering::Greater;
            }

            // Priority 5: Cheaper spells
            a_card.mana_value().cmp(&b_card.mana_value())
        });

        if !castable_spells.is_empty() {
            let (spell_idx, _spell) = castable_spells[0];

            if let Some(card) = state.hand.remove_card(spell_idx) {
                let card_name = card.name().to_string();
                let cost = get_mana_cost(&card);

                // Get for_creature for Cavern of Souls handling
                let for_creature = match &card {
                    Card::Creature(c) => Some(c),
                    _ => None,
                };

                if mana::tap_lands_for_cost(cost, state, for_creature) {
                    match &card {
                        Card::Creature(_) => {
                            let _ = cards::cast_creature(state, &card, false);

                            // Process ETB triggers
                            let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                            if perm_idx < state.battlefield.permanents().len() {
                                let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                                let _ = cards::process_etb_triggers_verbose(state, &mut perm, db, verbose);
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
                        Card::Instant(_) | Card::Sorcery(_) | Card::Enchantment(_) | Card::Saga(_) => {
                            let _ = cards::cast_spell(state, &card, db);
                            if verbose {
                                println!("  [Cast] {}", card_name);
                            }
                        }
                    }

                    cast_any = true;
                } else {
                    // Put it back if we can't pay
                    state.hand.add_card(card);
                }
            }
        }
    }
}

/// Execute main phase: play lands and cast spells
fn execute_main_phase(state: &mut GameState, db: &CardDatabase, verbose: bool) {
    // DO NOT tap lands here - TypeScript taps lands DURING casting, not before
    // This means can_cast_spell checks untapped lands, and cast_spell taps them
    main_phase(state, db, verbose);
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

