//! Turn 4 Combo Failure Analysis
//!
//! Analyzes why the combo couldn't execute on turn 4 across many simulations.
//!
//! The mana check point is: turn 4 main phase, after the land drop, before any
//! mana is spent. All color questions use the real mana logic
//! (`mana::can_tap_for_mana` / `mana::can_cast_spell`), so conditional lands
//! (Verges, Starting Town, Multiversal Passage, Pit of Offerings) are handled
//! the same way the engine handles them during play.

use crate::card::{Card, CardDatabase};
use crate::game::state::GameState;
use crate::game::mana;
use crate::game::cards::calculate_two_turn_combo_damage;
use std::collections::HashMap;
use std::fmt;

/// Why {2}{U}{B} cannot be paid even though 4+ mana is available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrongManaKind {
    /// No land can produce U
    NoBlue,
    /// No land can produce B
    NoBlack,
    /// No land can produce U and no land can produce B
    NoBlueNoBlack,
    /// Some land makes U and some land makes B, but the full cost
    /// cannot be paid at the same time (e.g. one dual is the only
    /// source of both colors)
    CannotPayTogether,
}

impl fmt::Display for WrongManaKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBlue => write!(f, "No U"),
            Self::NoBlack => write!(f, "No B"),
            Self::NoBlueNoBlack => write!(f, "No U and no B"),
            Self::CannotPayTogether => write!(f, "Colors present, cannot pay together"),
        }
    }
}

/// Reasons why the combo couldn't execute on turn 4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureReason {
    // Primary blockers (mutually exclusive - pick the first that applies).
    // Combo pieces are checked first, then whether the mana can cast them.
    SpiderManNotAvailable,       // No Spider-Man in hand, and no Beseech that can fetch it
    NoTargetInGraveyard,         // No Bringer (or Ardyn + another creature) in the graveyard
    PiecesReadyTooFewMana,       // Pieces ready, fewer than 4 untapped mana
    PiecesReadyWrongColors,      // Pieces ready, 4+ mana, but the colors cannot cast it
    InsufficientDamage,          // Castable, but two-turn damage < opponent life

    // Success case
    ComboAvailable,              // Combo could have fired on turn 4
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpiderManNotAvailable => write!(f, "No Spider-Man (or Beseech for it)"),
            Self::NoTargetInGraveyard => write!(f, "No combo target in graveyard"),
            Self::PiecesReadyTooFewMana => write!(f, "Pieces ready, fewer than 4 mana"),
            Self::PiecesReadyWrongColors => write!(f, "Pieces ready, wrong colors"),
            Self::InsufficientDamage => write!(f, "Insufficient damage"),
            Self::ComboAvailable => write!(f, "✓ Combo available"),
        }
    }
}

/// Result of the mana check at a main-phase check point
/// (after the land drop, before any mana is spent)
#[derive(Debug, Clone, Default)]
pub struct ManaCheck {
    /// Total mana available (untapped lands)
    pub total_mana: u32,
    /// Some land can tap for U
    pub has_blue: bool,
    /// Some land can tap for B
    pub has_black: bool,
    /// {2}{U}{B} (Spider-Man's real cost) can be paid right now
    pub can_pay_spider_man: bool,
    /// Set when total_mana >= 4 but {2}{U}{B} cannot be paid
    pub wrong_mana: Option<WrongManaKind>,
    /// Spells in hand with mana value <= total_mana that still cannot be cast
    /// (their colors cannot be paid)
    pub color_locked: Vec<String>,
}

/// Why ready combo pieces could not be cast
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComboBlock {
    TooFewMana,
    WrongColors,
}

/// Combo pieces at a main-phase check point (after the land drop, before any
/// mana is spent)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComboCheck {
    /// A combo target is in the graveyard (Bringer, or Ardyn + another
    /// creature) AND Spider-Man is in hand or a Beseech can fetch and
    /// free-cast it (Beseech in hand, bargain fodder, Spider-Man in library)
    pub pieces_ready: bool,
    /// Pieces ready and the mana can cast Spider-Man (or the Beseech)
    pub castable: bool,
    /// Pieces ready but not castable: why
    pub blocked: Option<ComboBlock>,
}

/// Check the combo pieces and whether the mana can cast them right now.
pub fn check_combo(state: &GameState) -> ComboCheck {
    let in_hand = |name: &str| state.hand.cards().iter().find(|c| c.name() == name);
    let gy = state.graveyard.cards();
    let bringer_in_gy = gy.iter().any(|c| c.name() == "Bringer of the Last Gift");
    let ardyn_in_gy = gy.iter().any(|c| c.name() == "Ardyn, the Usurper");
    let other_creature_in_gy = gy.iter()
        .any(|c| matches!(c, Card::Creature(_)) && c.name() != "Ardyn, the Usurper");
    let target_in_gy = bringer_in_gy || (ardyn_in_gy && other_creature_in_gy);

    let spidey = in_hand("Superior Spider-Man");
    let beseech = in_hand("Beseech the Mirror").filter(|_| {
        crate::game::cards::beseech_bargain_fodder(state).is_some()
            && state.library.cards().iter().any(|c| c.name() == "Superior Spider-Man")
    });
    let pieces_ready = target_in_gy && (spidey.is_some() || beseech.is_some());
    if !pieces_ready {
        return ComboCheck::default();
    }
    let castable = spidey.map_or(false, |c| mana::can_cast_spell(c, state))
        || beseech.map_or(false, |c| mana::can_cast_spell(c, state));
    let untapped = state.battlefield.permanents().iter()
        .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
        .count();
    let blocked = if castable {
        None
    } else if untapped < 4 {
        Some(ComboBlock::TooFewMana)
    } else {
        Some(ComboBlock::WrongColors)
    };
    ComboCheck { pieces_ready, castable, blocked }
}

/// Results from analyzing a single game at turn 4
#[derive(Debug, Clone)]
pub struct Turn4Analysis {
    pub primary_failure: FailureReason,
    pub mana: ManaCheck,
    pub combo: ComboCheck,
}

/// Aggregate results from analyzing many games
#[derive(Debug, Default)]
pub struct AnalysisResults {
    pub failure_counts: HashMap<FailureReason, usize>,
    /// Games with the combo pieces ready at the turn 4 check point
    pub pieces_ready_games: usize,
    /// ...of those, games where the mana could cast it
    pub pieces_castable_games: usize,
    /// ...of those, games blocked by too few mana / wrong colors
    pub pieces_too_few_mana_games: usize,
    pub pieces_wrong_colors_games: usize,
    pub avg_mana: f64,
    /// % of games with U / B available at the check point
    pub color_availability: (f64, f64),
    /// Games with 4+ mana at the check point
    pub games_with_4plus_mana: usize,
    /// Games with 4+ mana that cannot pay {2}{U}{B}
    pub wrong_mana_games: usize,
    pub wrong_mana_counts: HashMap<WrongManaKind, usize>,
    /// Games with at least one color-locked spell in hand
    pub color_locked_games: usize,
    /// Per-card counts of color-locked spells (game counts, descending)
    pub color_locked_cards: Vec<(String, usize)>,
}

/// Simulate the turn's land drop without spending any mana.
/// Uses the engine's own land choice (DecisionEngine) and the real
/// `play_land` tapped/untapped logic.
pub fn simulate_land_drop(state: &mut GameState) {
    use crate::simulation::decisions::DecisionEngine;

    if state.land_played_this_turn {
        return;
    }
    let hand_cards = state.hand.cards().to_vec();
    if let Some(land_idx) = DecisionEngine::choose_land_to_play(&hand_cards, state) {
        if let Some(card) = state.hand.remove_card(land_idx) {
            let _ = crate::game::cards::play_land(state, &card, false);
        }
    }
}

/// Advance a state from "end of previous turn" to this turn's mana check
/// point: untap, upkeep, draw, saga advancement, then the land drop.
/// No mana is spent and no RNG is consumed.
pub fn advance_to_mana_check_point(state: &mut GameState) {
    use crate::game::turns::{start_turn, draw_phase, upkeep_phase, precombat_main_phase_start};

    start_turn(state);
    upkeep_phase(state);
    state.phase = crate::game::state::Phase::Draw;
    draw_phase(state);
    state.phase = crate::game::state::Phase::Main1;
    precombat_main_phase_start(state, false);
    simulate_land_drop(state);
}

/// Run the mana check at the current state (expected to be at a main-phase
/// check point: after the land drop, before any mana is spent).
/// Uses Spider-Man's real cost from the card database and the real mana logic.
pub fn check_mana(state: &GameState, db: &CardDatabase) -> ManaCheck {
    let spider_man = db
        .get_card("Superior Spider-Man")
        .expect("Superior Spider-Man must exist in cards.json");

    // Total mana = untapped lands (every land taps for exactly 1 mana)
    let total_mana = state.battlefield.permanents().iter()
        .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
        .count() as u32;

    // Union of colors the untapped lands can actually produce right now
    let mut has_blue = false;
    let mut has_black = false;
    for permanent in state.battlefield.permanents() {
        if matches!(permanent.card, Card::Land(_)) && !permanent.tapped {
            let colors = mana::can_tap_for_mana(permanent, state, None);
            if colors.has_blue() { has_blue = true; }
            if colors.has_black() { has_black = true; }
        }
    }

    // Can we pay Spider-Man's real cost ({2}{U}{B}) with the real mana logic?
    let can_pay_spider_man = mana::can_cast_spell(&spider_man, state);

    let wrong_mana = if total_mana >= 4 && !can_pay_spider_man {
        Some(match (has_blue, has_black) {
            (false, false) => WrongManaKind::NoBlueNoBlack,
            (false, true) => WrongManaKind::NoBlue,
            (true, false) => WrongManaKind::NoBlack,
            (true, true) => WrongManaKind::CannotPayTogether,
        })
    } else {
        None
    };

    // Color-locked spells: mana value fits in total mana, but the colors
    // cannot be paid (every land produces exactly 1 mana, so a castability
    // failure at mana value <= total_mana is a color failure)
    let mut color_locked = Vec::new();
    for card in state.hand.cards() {
        if matches!(card, Card::Land(_)) {
            continue;
        }
        if card.mana_value() <= total_mana && !mana::can_cast_spell(card, state) {
            color_locked.push(card.name().to_string());
        }
    }

    ManaCheck {
        total_mana,
        has_blue,
        has_black,
        can_pay_spider_man,
        wrong_mana,
        color_locked,
    }
}

/// Analyze the game state at the turn 4 mana check point
/// (after the land drop, before any mana is spent)
pub fn analyze_turn4_state(state: &GameState, db: &CardDatabase) -> Turn4Analysis {
    let mana = check_mana(state, db);

    let combo = check_combo(state);

    // Expected damage over the next two turns (matches the combo gate)
    let combo_damage = calculate_two_turn_combo_damage(state);

    // Primary failure: combo pieces first, then whether the mana can cast them
    let spider_available = state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man")
        || (state.hand.cards().iter().any(|c| c.name() == "Beseech the Mirror")
            && crate::game::cards::beseech_bargain_fodder(state).is_some()
            && state.library.cards().iter().any(|c| c.name() == "Superior Spider-Man"));
    let primary_failure = if !spider_available {
        FailureReason::SpiderManNotAvailable
    } else if !combo.pieces_ready {
        FailureReason::NoTargetInGraveyard
    } else if combo.blocked == Some(ComboBlock::TooFewMana) {
        FailureReason::PiecesReadyTooFewMana
    } else if combo.blocked == Some(ComboBlock::WrongColors) {
        FailureReason::PiecesReadyWrongColors
    } else if combo_damage < state.opponent_life as u32 {
        FailureReason::InsufficientDamage
    } else {
        FailureReason::ComboAvailable
    };

    Turn4Analysis {
        primary_failure,
        mana,
        combo,
    }
}

/// Run a game to turn 4 only (for analysis)
/// Analyzes the state at the turn 4 mana check point:
/// main phase, after the land drop, before any mana is spent.
pub fn run_game_to_turn4(
    deck: &[Card],
    seed: u64,
    db: &CardDatabase,
) -> Turn4Analysis {
    use crate::simulation::mulligan::resolve_mulligans;
    use crate::rng::GameRng;
    use crate::simulation::engine::execute_turn;

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

    let opening_hand = resolve_mulligans(
        &mut library_cards,
        &mut rng,
        deck.iter().filter(|c| matches!(c, Card::Land(_))).count(),
        deck.len(),
    );

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

    // Turn 4: untap, upkeep, draw, saga advancement, land drop - then analyze
    advance_to_mana_check_point(&mut state);

    analyze_turn4_state(&state, db)
}

/// Aggregate results from multiple analyses
pub fn aggregate_results(analyses: &[Turn4Analysis]) -> AnalysisResults {
    let mut results = AnalysisResults::default();

    if analyses.is_empty() {
        return results;
    }

    let mut total_mana = 0u64;
    let mut blue_count = 0usize;
    let mut black_count = 0usize;
    let mut locked_card_counts: HashMap<String, usize> = HashMap::new();

    for analysis in analyses {
        *results.failure_counts.entry(analysis.primary_failure).or_insert(0) += 1;
        if analysis.combo.pieces_ready {
            results.pieces_ready_games += 1;
            if analysis.combo.castable { results.pieces_castable_games += 1; }
            match analysis.combo.blocked {
                Some(ComboBlock::TooFewMana) => results.pieces_too_few_mana_games += 1,
                Some(ComboBlock::WrongColors) => results.pieces_wrong_colors_games += 1,
                None => {}
            }
        }
        total_mana += analysis.mana.total_mana as u64;
        if analysis.mana.has_blue { blue_count += 1; }
        if analysis.mana.has_black { black_count += 1; }
        if analysis.mana.total_mana >= 4 {
            results.games_with_4plus_mana += 1;
        }
        if let Some(kind) = analysis.mana.wrong_mana {
            results.wrong_mana_games += 1;
            *results.wrong_mana_counts.entry(kind).or_insert(0) += 1;
        }
        if !analysis.mana.color_locked.is_empty() {
            results.color_locked_games += 1;
            // Count each card once per game
            let mut seen = std::collections::HashSet::new();
            for name in &analysis.mana.color_locked {
                if seen.insert(name.clone()) {
                    *locked_card_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let n = analyses.len() as f64;
    results.avg_mana = total_mana as f64 / n;
    results.color_availability = (
        blue_count as f64 / n * 100.0,
        black_count as f64 / n * 100.0,
    );

    let mut locked: Vec<(String, usize)> = locked_card_counts.into_iter().collect();
    locked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    results.color_locked_cards = locked;

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardDatabase;
    use crate::game::zones::Permanent;

    fn db() -> CardDatabase {
        CardDatabase::from_file("cards.json").expect("Failed to load cards")
    }

    fn add_land(state: &mut GameState, db: &CardDatabase, name: &str) {
        let card = db.get_card(name).expect(name);
        state.battlefield.add_permanent(Permanent::new(card, 1));
    }

    #[test]
    fn test_combo_pieces_ready_and_castable() {
        let db = db();
        let mut state = GameState::new();
        for l in ["Watery Grave", "Watery Grave", "Swamp", "Island"] { add_land(&mut state, &db, l); }
        state.hand.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        let c = check_combo(&state);
        assert!(c.pieces_ready && c.castable);
        assert_eq!(c.blocked, None);
    }

    #[test]
    fn test_combo_pieces_ready_but_too_few_mana() {
        let db = db();
        let mut state = GameState::new();
        for l in ["Watery Grave", "Swamp", "Island"] { add_land(&mut state, &db, l); }
        state.hand.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        let c = check_combo(&state);
        assert!(c.pieces_ready && !c.castable);
        assert_eq!(c.blocked, Some(ComboBlock::TooFewMana));
    }

    #[test]
    fn test_combo_pieces_ready_via_beseech() {
        let db = db();
        let mut state = GameState::new();
        for l in ["Swamp", "Swamp", "Swamp", "Island"] { add_land(&mut state, &db, l); }
        state.battlefield.add_permanent(Permanent::new(db.get_card("Does Machines").expect("DM"), 1));
        state.hand.add_card(db.get_card("Beseech the Mirror").expect("Beseech"));
        state.library.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        let c = check_combo(&state);
        assert!(c.pieces_ready && c.castable, "Beseech with fodder fetches and free-casts Spider-Man");
    }

    #[test]
    fn test_combo_pieces_not_ready_without_target() {
        let db = db();
        let mut state = GameState::new();
        for l in ["Watery Grave", "Watery Grave", "Swamp", "Island"] { add_land(&mut state, &db, l); }
        state.hand.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        let c = check_combo(&state);
        assert!(!c.pieces_ready);
    }

    #[test]
    fn test_four_lands_with_u_and_b_is_not_wrong_mana() {
        let db = db();
        let mut state = GameState::new();
        add_land(&mut state, &db, "Island");
        add_land(&mut state, &db, "Swamp");
        add_land(&mut state, &db, "Swamp");
        add_land(&mut state, &db, "Swamp");

        let check = check_mana(&state, &db);
        assert_eq!(check.total_mana, 4);
        assert!(check.can_pay_spider_man, "{{2}}{{U}}{{B}} must be payable");
        assert!(check.wrong_mana.is_none(), "No wrong mana with U and B");
    }

    #[test]
    fn test_four_lands_without_u_is_wrong_mana_no_blue() {
        let db = db();
        let mut state = GameState::new();
        add_land(&mut state, &db, "Swamp");
        add_land(&mut state, &db, "Swamp");
        add_land(&mut state, &db, "Swamp");
        add_land(&mut state, &db, "Swamp");

        let check = check_mana(&state, &db);
        assert_eq!(check.total_mana, 4);
        assert!(!check.can_pay_spider_man);
        assert_eq!(check.wrong_mana, Some(WrongManaKind::NoBlue));
    }

    #[test]
    fn test_blazemire_verge_without_swamp_or_mountain_is_not_red() {
        let db = db();
        let mut state = GameState::new();
        // Blazemire Verge makes B only without a Swamp or Mountain in play
        add_land(&mut state, &db, "Blazemire Verge");
        add_land(&mut state, &db, "Island");
        add_land(&mut state, &db, "Island");
        add_land(&mut state, &db, "Island");

        // Fear of Missing Out costs {1}{R}: no red available, so color-locked
        let fomo = db.get_card("Fear of Missing Out").expect("FoMO");
        state.hand.add_card(fomo);

        let check = check_mana(&state, &db);
        assert_eq!(check.total_mana, 4);
        assert!(
            check.color_locked.iter().any(|n| n == "Fear of Missing Out"),
            "FoMO must be color-locked: Blazemire Verge is not R without a Swamp or Mountain"
        );

        // With a Swamp instead of an Island, the Verge makes R and FoMO is castable
        let mut state2 = GameState::new();
        add_land(&mut state2, &db, "Blazemire Verge");
        add_land(&mut state2, &db, "Swamp");
        let fomo = db.get_card("Fear of Missing Out").expect("FoMO");
        state2.hand.add_card(fomo);
        let check2 = check_mana(&state2, &db);
        assert!(
            check2.color_locked.is_empty(),
            "FoMO must be castable once the Verge makes R"
        );
    }

    #[test]
    fn test_color_locked_spell_in_hand() {
        let db = db();
        let mut state = GameState::new();
        // 4 lands, U and B available, but only one B source:
        // Beseech the Mirror {1}{B}{B}{B} cannot be paid
        add_land(&mut state, &db, "Island");
        add_land(&mut state, &db, "Island");
        add_land(&mut state, &db, "Island");
        add_land(&mut state, &db, "Swamp");

        let beseech = db.get_card("Beseech the Mirror").expect("Beseech");
        state.hand.add_card(beseech);

        let check = check_mana(&state, &db);
        assert_eq!(check.total_mana, 4);
        assert_eq!(check.color_locked, vec!["Beseech the Mirror".to_string()]);
        // Spider-Man itself is payable here (U + B + 2 generic)
        assert!(check.can_pay_spider_man);
        assert!(check.wrong_mana.is_none());
    }

    #[test]
    fn test_one_dual_cannot_pay_both_colors_together() {
        let db = db();
        let mut state = GameState::new();
        // Watery Grave is the only U and the only B source:
        // U and B are both "available", but not at the same time
        add_land(&mut state, &db, "Watery Grave");
        add_land(&mut state, &db, "Forest");
        add_land(&mut state, &db, "Forest");
        add_land(&mut state, &db, "Forest");

        let check = check_mana(&state, &db);
        assert_eq!(check.total_mana, 4);
        assert!(check.has_blue);
        assert!(check.has_black);
        assert!(!check.can_pay_spider_man);
        assert_eq!(check.wrong_mana, Some(WrongManaKind::CannotPayTogether));
    }
}
