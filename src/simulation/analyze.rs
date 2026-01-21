//! Turn 4 Combo Failure Analysis
//!
//! Analyzes why the combo couldn't execute on turn 4 across many simulations.

use crate::card::{Card, CardDatabase};
use crate::game::state::GameState;
use crate::game::mana;
use crate::game::cards::calculate_combo_damage;
use std::collections::HashMap;
use std::fmt;

/// Reasons why the combo couldn't execute on turn 4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureReason {
    // Primary blockers (mutually exclusive - pick the first that applies)
    InsufficientLands,           // Less than 4 lands on battlefield
    MissingBlue,                 // Have 4+ lands but no blue mana
    MissingBlack,                // Have 4+ lands but no black mana
    MissingGreen,                // Have 4+ lands but no green mana
    SpiderManNotInHand,          // Spider-Man not in hand
    NoBringerInGraveyard,        // No Bringer in graveyard to copy
    NoTerrorInGraveyard,         // No Terror in graveyard for damage
    InsufficientDamage,          // Have all pieces but damage < 20
    
    // Success case
    ComboAvailable,              // Combo could have fired on turn 4
}

/// Secondary details about card locations
#[derive(Debug, Clone, Default)]
pub struct CardLocations {
    pub spider_man: CardLocation,
    pub bringer: CardLocation,
    pub terror: CardLocation,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CardLocation {
    pub in_hand: u32,
    pub in_graveyard: u32,
    pub on_battlefield: u32,
}

/// Results from analyzing a single game at turn 4
#[derive(Debug, Clone)]
pub struct Turn4Analysis {
    pub primary_failure: FailureReason,
    pub lands_count: u32,
    pub colors_available: (bool, bool, bool), // (U, B, G)
}

/// Aggregate results from analyzing many games
#[derive(Debug, Default)]
pub struct AnalysisResults {
    pub failure_counts: HashMap<FailureReason, usize>,
    pub avg_lands: f64,
    pub color_availability: (f64, f64, f64), // % of games with U, B, G available
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientLands => write!(f, "Insufficient lands (<4)"),
            Self::MissingBlue => write!(f, "Missing blue mana"),
            Self::MissingBlack => write!(f, "Missing black mana"),
            Self::MissingGreen => write!(f, "Missing green mana"),
            Self::SpiderManNotInHand => write!(f, "Spider-Man not in hand"),
            Self::NoBringerInGraveyard => write!(f, "No Bringer in graveyard"),
            Self::NoTerrorInGraveyard => write!(f, "No Terror in graveyard"),
            Self::InsufficientDamage => write!(f, "Insufficient damage (<20)"),
            Self::ComboAvailable => write!(f, "✓ Combo available"),
        }
    }
}

/// Analyze the game state at turn 4 to determine why combo couldn't fire
/// This should be called at the START of turn 4's main phase (after draw)
pub fn analyze_turn4_state(state: &GameState) -> Turn4Analysis {
    use crate::card::LandSubtype;

    // Count lands on battlefield
    let lands_on_battlefield = state.battlefield.permanents().iter()
        .filter(|p| matches!(p.card, Card::Land(_)))
        .count() as u32;

    // Check available colors from lands currently on battlefield
    let mut has_blue = false;
    let mut has_black = false;
    let mut has_green = false;

    for permanent in state.battlefield.permanents() {
        if matches!(permanent.card, Card::Land(_)) {
            let colors = mana::can_tap_for_mana(permanent, state, None);
            if colors.has_blue() { has_blue = true; }
            if colors.has_black() { has_black = true; }
            if colors.has_green() { has_green = true; }
        }
    }

    // Check if we have a land in hand that enters untapped on turn 4
    // This affects both land count and color availability
    let mut land_in_hand_untapped = false;
    let mut land_in_hand_colors = (false, false, false); // (U, B, G)

    for card in state.hand.cards() {
        if let Card::Land(land) = card {
            // Check if this land would enter untapped on turn 4
            let enters_tapped = match land.subtype {
                LandSubtype::Fastland => {
                    // Fastland enters tapped if we control 3+ other lands
                    lands_on_battlefield >= 3
                }
                LandSubtype::Town => {
                    // Starting Town enters tapped on turn 4+
                    state.turn > 3  // turn 4 = tapped
                }
                LandSubtype::Shock => {
                    // Shock lands can pay 2 life to enter untapped
                    state.life <= 2
                }
                _ => land.enters_tapped,
            };

            if !enters_tapped {
                land_in_hand_untapped = true;
                // Check what colors this land provides
                for color in &land.colors {
                    match color {
                        crate::card::ManaColor::Blue => land_in_hand_colors.0 = true,
                        crate::card::ManaColor::Black => land_in_hand_colors.1 = true,
                        crate::card::ManaColor::Green => land_in_hand_colors.2 = true,
                        _ => {}
                    }
                }
            }
        }
    }

    // Total available mana = lands on battlefield + (1 if untapped land in hand)
    let total_mana = lands_on_battlefield + if land_in_hand_untapped { 1 } else { 0 };

    // Colors available = battlefield colors + hand land colors (if untapped)
    if land_in_hand_untapped {
        has_blue = has_blue || land_in_hand_colors.0;
        has_black = has_black || land_in_hand_colors.1;
        has_green = has_green || land_in_hand_colors.2;
    }

    // Find card locations
    let mut locations = CardLocations::default();
    
    // Check hand
    for card in state.hand.cards() {
        match card.name() {
            "Superior Spider-Man" => locations.spider_man.in_hand += 1,
            "Bringer of the Last Gift" => locations.bringer.in_hand += 1,
            "Terror of the Peaks" => locations.terror.in_hand += 1,
            _ => {}
        }
    }
    
    // Check graveyard
    for card in state.graveyard.cards() {
        match card.name() {
            "Superior Spider-Man" => locations.spider_man.in_graveyard += 1,
            "Bringer of the Last Gift" => locations.bringer.in_graveyard += 1,
            "Terror of the Peaks" => locations.terror.in_graveyard += 1,
            _ => {}
        }
    }
    
    // Check battlefield
    for perm in state.battlefield.permanents() {
        match perm.card.name() {
            "Superior Spider-Man" => locations.spider_man.on_battlefield += 1,
            "Bringer of the Last Gift" => locations.bringer.on_battlefield += 1,
            "Terror of the Peaks" => locations.terror.on_battlefield += 1,
            _ => {}
        }
    }
    
    // Calculate expected damage
    let combo_damage = calculate_combo_damage(state);

    // Determine primary failure reason (in priority order)
    let primary_failure = determine_primary_failure(
        total_mana, has_blue, has_black, has_green,
        &locations, combo_damage, state.opponent_life,
    );

    Turn4Analysis {
        primary_failure,
        lands_count: total_mana,  // Total mana available (battlefield + playable land)
        colors_available: (has_blue, has_black, has_green),
    }
}

/// Determine the primary failure reason based on game state
fn determine_primary_failure(
    lands_count: u32,
    has_blue: bool,
    has_black: bool,
    has_green: bool,
    locations: &CardLocations,
    combo_damage: u32,
    opponent_life: i32,
) -> FailureReason {
    // Check in priority order - return first failure found

    // 1. Not enough lands
    if lands_count < 4 {
        return FailureReason::InsufficientLands;
    }

    // 2. Missing colors (Spider-Man costs UBG)
    if !has_blue {
        return FailureReason::MissingBlue;
    }
    if !has_black {
        return FailureReason::MissingBlack;
    }
    if !has_green {
        return FailureReason::MissingGreen;
    }

    // 3. Spider-Man not in hand
    if locations.spider_man.in_hand == 0 {
        return FailureReason::SpiderManNotInHand;
    }

    // 4. No Bringer in graveyard to copy
    if locations.bringer.in_graveyard == 0 {
        return FailureReason::NoBringerInGraveyard;
    }

    // 5. No Terror in graveyard for damage (need at least one for triggers)
    // Note: Terror on battlefield also works, so check both
    let has_terror_source = locations.terror.in_graveyard > 0
        || locations.terror.on_battlefield > 0;
    if !has_terror_source {
        return FailureReason::NoTerrorInGraveyard;
    }

    // 6. Not enough damage
    if combo_damage < opponent_life as u32 {
        return FailureReason::InsufficientDamage;
    }

    // All requirements met!
    FailureReason::ComboAvailable
}

/// Run a game to turn 4 only (for analysis)
/// Analyzes state at the START of turn 4 (after draw, before main phase)
pub fn run_game_to_turn4(
    deck: &[Card],
    seed: u64,
    db: &CardDatabase,
) -> Turn4Analysis {
    use crate::simulation::mulligan::resolve_mulligans;
    use crate::rng::GameRng;
    use crate::simulation::engine::execute_turn;
    use crate::game::turns::{start_turn, draw_phase, upkeep_phase, precombat_main_phase_start};

    let mut rng = GameRng::new(Some(seed));
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
    for card in opening_hand {
        state.hand.add_card(card);
    }

    // Run turns 1-3 fully
    for _ in 0..3 {
        execute_turn(&mut state, db, false, &mut rng);
    }

    // Turn 4: only do start_turn (untap), upkeep, draw, and precombat main start - then analyze
    // This gives us the state at the START of turn 4's main phase (after saga advancement)
    start_turn(&mut state);
    upkeep_phase(&mut state);
    draw_phase(&mut state);
    precombat_main_phase_start(&mut state, false);

    // Analyze state at START of turn 4 main phase
    // All lands are untapped (from start_turn), we've drawn for the turn, sagas advanced
    analyze_turn4_state(&state)
}

/// Aggregate results from multiple analyses
pub fn aggregate_results(analyses: &[Turn4Analysis]) -> AnalysisResults {
    let mut results = AnalysisResults {
        failure_counts: HashMap::new(),
        avg_lands: 0.0,
        color_availability: (0.0, 0.0, 0.0),
    };

    if analyses.is_empty() {
        return results;
    }

    let mut total_lands = 0u64;
    let mut blue_count = 0usize;
    let mut black_count = 0usize;
    let mut green_count = 0usize;

    for analysis in analyses {
        *results.failure_counts.entry(analysis.primary_failure).or_insert(0) += 1;
        total_lands += analysis.lands_count as u64;
        if analysis.colors_available.0 { blue_count += 1; }
        if analysis.colors_available.1 { black_count += 1; }
        if analysis.colors_available.2 { green_count += 1; }
    }

    let n = analyses.len() as f64;
    results.avg_lands = total_lands as f64 / n;
    results.color_availability = (
        blue_count as f64 / n * 100.0,
        black_count as f64 / n * 100.0,
        green_count as f64 / n * 100.0,
    );

    results
}

