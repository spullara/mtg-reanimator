use crate::card::{Card, CardDatabase};
use crate::game::state::GameState;
use crate::game::turns::{start_turn, draw_phase, upkeep_phase, end_phase};
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
pub fn simulate_combat(state: &mut GameState) -> u32 {
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
    
    total_damage
}

/// Execute a single turn: untap -> draw -> main -> combat -> end
pub fn execute_turn(state: &mut GameState, _db: &CardDatabase) -> u32 {
    // Start turn: increment turn counter, untap, reset land drop
    start_turn(state);
    
    // Upkeep phase
    upkeep_phase(state);
    
    // Draw phase
    state.phase = crate::game::state::Phase::Draw;
    draw_phase(state);
    
    // Main phase 1
    state.phase = crate::game::state::Phase::Main1;
    // Main phase logic would go here (casting spells, playing lands)
    // For now, we just pass through
    
    // Combat phase
    state.phase = crate::game::state::Phase::Combat;
    let combat_damage = simulate_combat(state);
    
    // Main phase 2
    state.phase = crate::game::state::Phase::Main2;
    // Additional spell casting could happen here
    
    // End phase
    state.phase = crate::game::state::Phase::End;
    end_phase(state);
    
    combat_damage
}

/// Run a complete game simulation
pub fn run_game(
    deck: &[Card],
    seed: u64,
    _db: &CardDatabase,
) -> GameResult {
    let mut rng = GameRng::new(Some(seed));
    
    // Initialize game state
    let mut state = GameState::new();
    
    // Shuffle deck into library
    let mut shuffled_deck = deck.to_vec();
    rng.shuffle(&mut shuffled_deck);
    for card in shuffled_deck {
        state.library.add_card(card);
    }
    
    // Determine if on play or draw (50/50)
    state.on_the_play = rng.random() < 0.5;
    
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
    for card in opening_hand {
        state.hand.add_card(card);
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
        let combat_damage = execute_turn(&mut state, _db);
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
        let damage = simulate_combat(&mut state);
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
        
        let damage = simulate_combat(&mut state);
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
        
        let damage = simulate_combat(&mut state);
        assert_eq!(damage, 0); // Can't attack due to summoning sickness
        assert_eq!(state.opponent_life, 20);
    }
}

