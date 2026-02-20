use crate::card::{Card, CardDatabase, LandSubtype};
use crate::rng::GameRng;
use crate::simulation::mulligan::bo1_opening_hand;
use rayon::prelude::*;

#[derive(Clone, Debug)]
struct Permanent {
    name: String,
    is_land: bool,
    is_creature: bool,
    is_earthbent: bool,
    has_summoning_sickness: bool,
    is_tapped: bool,
    abilities: Vec<String>,
    is_basic: bool,
}

struct ManaGameResult {
    turn_mana: Vec<usize>,
    turn_creatures: Vec<usize>,
    turn_lands: Vec<usize>,
    mana_dork_turn_1: bool,
}

pub struct ManaSimResults {
    pub num_games: usize,
    pub max_turns: usize,
    pub turn_mana_values: Vec<Vec<usize>>,
    pub turn_creature_values: Vec<Vec<usize>>,
    pub turn_land_values: Vec<Vec<usize>>,
    pub mana_dork_turn_1_count: usize,
}

fn is_mana_relevant(card: &Card) -> bool {
    match card {
        Card::Creature(c) => c.abilities.iter().any(|a| {
            a == "tap_for_green" || a == "tap_plus_permanent_for_any_color"
                || a == "creature_mana_tap_bonus_green"
        }),
        Card::Enchantment(c) => c.abilities.iter().any(|a| {
            a == "etb_earthbend_2" || a == "etb_search_basic_land_tapped"
        }),
        Card::Sorcery(c) => c.abilities.iter().any(|a| a == "search_land_or_creature_with_evidence"),
        _ => false,
    }
}

fn card_abilities(card: &Card) -> Vec<String> {
    match card {
        Card::Creature(c) => c.abilities.clone(),
        Card::Enchantment(c) => c.abilities.clone(),
        Card::Sorcery(c) => c.abilities.clone(),
        _ => vec![],
    }
}

fn count_available_mana(battlefield: &[Permanent]) -> usize {
    let mut total: usize = 0;
    let bc = battlefield.iter()
        .filter(|p| p.abilities.iter().any(|a| a == "creature_mana_tap_bonus_green"))
        .count();
    let mut creatures_tap: usize = 0;
    for p in battlefield {
        if p.is_land && !p.is_tapped { total += 1; if p.is_earthbent { creatures_tap += 1; } }
    }
    for p in battlefield {
        if !p.is_land && p.is_creature && !p.has_summoning_sickness
            && p.abilities.iter().any(|a| a == "tap_for_green")
        { total += 1; creatures_tap += 1; }
    }
    let non_mana = battlefield.iter().filter(|p| {
        !p.is_land && p.is_creature
            && !p.abilities.iter().any(|a| a == "tap_for_green"
                || a == "tap_plus_permanent_for_any_color")
    }).count();
    let gp = battlefield.iter().filter(|p| {
        p.is_creature && !p.has_summoning_sickness
            && p.abilities.iter().any(|a| a == "tap_plus_permanent_for_any_color")
    }).count();
    let gp_paired = gp.min(non_mana);
    total += gp_paired;
    creatures_tap += gp_paired;

    total += creatures_tap * bc;
    total
}

fn count_untapped_mana(bf: &[Permanent]) -> usize {
    bf.iter().filter(|p| p.is_land && !p.is_tapped).count()
        + bf.iter().filter(|p| {
            !p.is_land && p.is_creature && !p.has_summoning_sickness && !p.is_tapped
                && p.abilities.iter().any(|a| a == "tap_for_green")
        }).count()
}

fn pay_mana(bf: &mut [Permanent], cost: usize) -> bool {
    if count_untapped_mana(bf) < cost { return false; }
    let mut rem = cost;
    for p in bf.iter_mut() {
        if rem == 0 { break; }
        if p.is_land && !p.is_tapped && !p.is_earthbent { p.is_tapped = true; rem -= 1; }
    }
    for p in bf.iter_mut() {
        if rem == 0 { break; }
        if p.is_land && !p.is_tapped && p.is_earthbent { p.is_tapped = true; rem -= 1; }
    }
    for p in bf.iter_mut() {
        if rem == 0 { break; }
        if !p.is_land && p.is_creature && !p.has_summoning_sickness && !p.is_tapped
            && p.abilities.iter().any(|a| a == "tap_for_green")
        { p.is_tapped = true; rem -= 1; }
    }
    rem == 0
}

fn has_basic_land(bf: &[Permanent]) -> bool {
    bf.iter().any(|p| p.is_land && p.is_basic)
}

fn earthbend(bf: &mut Vec<Permanent>, count: usize) {
    let mut rem = count;
    for p in bf.iter_mut() {
        if rem == 0 { break; }
        if p.is_land && !p.is_earthbent { p.is_earthbent = true; p.is_creature = true; rem -= 1; }
    }
}

fn play_land_from_hand(hand: &mut Vec<Card>, bf: &mut Vec<Permanent>) -> bool {
    let has_basic = has_basic_land(bf);
    let mut best: Option<(usize, bool)> = None;
    for (i, card) in hand.iter().enumerate() {
        if let Card::Land(l) = card {
            let tapped = if l.base.name == "Ba Sing Se" { !has_basic } else { l.enters_tapped };
            match &best {
                None => best = Some((i, tapped)),
                Some((_, bt)) => { if !tapped && *bt { best = Some((i, tapped)); } }
            }
        }
    }
    if let Some((idx, _)) = best {
        let card = hand.remove(idx);
        if let Card::Land(l) = &card {
            let tapped = if l.base.name == "Ba Sing Se" { !has_basic_land(bf) } else { l.enters_tapped };
            bf.push(Permanent {
                name: l.base.name.clone(), is_land: true, is_creature: false,
                is_earthbent: false, has_summoning_sickness: false, is_tapped: tapped,
                abilities: vec![], is_basic: l.subtype == LandSubtype::Basic,
            });
        }
        true
    } else { false }
}


fn play_spells(hand: &mut Vec<Card>, bf: &mut Vec<Permanent>, library: &mut Vec<Card>, _db: &CardDatabase) -> bool {
    let mut played_any = false;
    loop {
        let available = count_untapped_mana(bf);
        if available == 0 { break; }
        let mut best: Option<(usize, u32)> = None;
        for (i, card) in hand.iter().enumerate() {
            if !is_mana_relevant(card) { continue; }
            let mv = card.mana_value();
            if mv as usize <= available {
                match &best {
                    None => best = Some((i, mv)),
                    Some((_, bmv)) => { if mv < *bmv { best = Some((i, mv)); } }
                }
            }
        }
        if let Some((idx, mv)) = best {
            let card = hand.remove(idx);
            let abilities = card_abilities(&card);
            let card_name = card.name().to_string();
            pay_mana(bf, mv as usize);
            if let Card::Creature(_) = &card {
                bf.push(Permanent {
                    name: card_name.clone(), is_land: false, is_creature: true,
                    is_earthbent: false, has_summoning_sickness: true, is_tapped: false,
                    abilities: abilities.clone(), is_basic: false,
                });
            }
            for ability in &abilities {
                match ability.as_str() {
                    "etb_earthbend_1" => earthbend(bf, 1),
                    "etb_earthbend_2" => earthbend(bf, 2),
                    "etb_search_basic_land_tapped" => {
                        if let Some(pos) = library.iter().position(|c| {
                            matches!(c, Card::Land(l) if l.subtype == LandSubtype::Basic)
                        }) {
                            library.remove(pos);
                            bf.push(Permanent {
                                name: "Forest".to_string(), is_land: true, is_creature: false,
                                is_earthbent: false, has_summoning_sickness: false,
                                is_tapped: true, abilities: vec![], is_basic: true,
                            });
                        }
                    }
                    "search_land_or_creature_with_evidence" => {
                        let mut found = false;
                        if let Some(pos) = library.iter().position(|c| {
                            matches!(c, Card::Creature(_)) && is_mana_relevant(c)
                        }) {
                            hand.push(library.remove(pos));
                            found = true;
                        }
                        if !found {
                            if let Some(pos) = library.iter().position(|c| matches!(c, Card::Land(_))) {
                                hand.push(library.remove(pos));
                            }
                        }
                    }
                    _ => {}
                }
            }
            played_any = true;
        } else { break; }
    }
    played_any
}

fn run_mana_game(deck: &[Card], seed: u64, db: &CardDatabase, max_turns: usize) -> ManaGameResult {
    let mut rng = GameRng::new(Some(seed));
    let mut library: Vec<Card> = deck.to_vec();
    rng.shuffle(&mut library);
    let deck_land_count = deck.iter().filter(|c| matches!(c, Card::Land(_))).count();
    let hand_cards = bo1_opening_hand(&mut library, &mut rng, deck_land_count, deck.len());
    let mut hand: Vec<Card> = hand_cards;

    // Mulligan logic: check for unkeepable hands
    let land_count = hand.iter().filter(|c| matches!(c, Card::Land(_))).count();
    let has_mana_dork = hand.iter().any(|c| {
        if let Card::Creature(cr) = c {
            cr.abilities.iter().any(|a| a == "tap_for_green" || a == "tap_plus_permanent_for_any_color")
        } else {
            false
        }
    });
    let should_mulligan = land_count == 0 || (land_count == 1 && !has_mana_dork);
    if should_mulligan {
        // Put hand back into library, shuffle, draw 6
        library.extend(hand.drain(..));
        rng.shuffle(&mut library);
        for _ in 0..6 {
            if let Some(card) = library.pop() {
                hand.push(card);
            }
        }
    }

    let mut bf: Vec<Permanent> = Vec::new();
    let mut turn_mana = Vec::with_capacity(max_turns);
    let mut turn_creatures = Vec::with_capacity(max_turns);
    let mut turn_lands = Vec::with_capacity(max_turns);
    let mut mana_dork_turn_1 = false;

    for turn in 1..=max_turns {
        // Untap all
        for p in bf.iter_mut() { p.is_tapped = false; }
        // Clear summoning sickness on existing creatures
        for p in bf.iter_mut() { p.has_summoning_sickness = false; }
        // Draw (skip turn 1 — on the play)
        if turn > 1 {
            if let Some(card) = library.pop() { hand.push(card); }
        }
        // Play a land
        play_land_from_hand(&mut hand, &mut bf);
        // Record mana available THIS turn (before spending)
        let mana = count_available_mana(&bf);
        // Play mana-producing spells (advances game state for future turns)
        play_spells(&mut hand, &mut bf, &mut library, db);
        // Track turn-1 mana dork
        if turn == 1 {
            mana_dork_turn_1 = bf.iter().any(|p| {
                !p.is_land && p.is_creature
                    && p.abilities.iter().any(|a| a == "tap_for_green")
            });
        }
        // Record stats
        let creatures = bf.iter().filter(|p| p.is_creature && !p.is_land).count();
        let lands = bf.iter().filter(|p| p.is_land).count();
        turn_mana.push(mana);
        turn_creatures.push(creatures);
        turn_lands.push(lands);
    }
    ManaGameResult { turn_mana, turn_creatures, turn_lands, mana_dork_turn_1 }
}

pub fn run_mana_simulation(deck: &[Card], num_games: usize, max_turns: usize, db: &CardDatabase) -> ManaSimResults {
    let results: Vec<ManaGameResult> = (0..num_games)
        .into_par_iter()
        .map(|i| run_mana_game(deck, i as u64, db, max_turns))
        .collect();

    let mut turn_mana_values = vec![Vec::with_capacity(num_games); max_turns];
    let mut turn_creature_values = vec![Vec::with_capacity(num_games); max_turns];
    let mut turn_land_values = vec![Vec::with_capacity(num_games); max_turns];
    let mut mana_dork_turn_1_count = 0;

    for result in &results {
        if result.mana_dork_turn_1 { mana_dork_turn_1_count += 1; }
        for t in 0..max_turns {
            if t < result.turn_mana.len() {
                turn_mana_values[t].push(result.turn_mana[t]);
                turn_creature_values[t].push(result.turn_creatures[t]);
                turn_land_values[t].push(result.turn_lands[t]);
            }
        }
    }

    ManaSimResults {
        num_games, max_turns, turn_mana_values, turn_creature_values,
        turn_land_values, mana_dork_turn_1_count,
    }
}

fn percentile(sorted: &[usize], pct: f64) -> usize {
    if sorted.is_empty() { return 0; }
    let idx = ((sorted.len() as f64 - 1.0) * pct / 100.0).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub fn print_mana_results(results: &ManaSimResults, deck_file: &str, deck_size: usize, land_count: usize) {
    println!("\n=== Mana Production Simulation ===");
    println!("Deck: {} ({} cards, {} lands)", deck_file, deck_size, land_count);
    println!("Games: {} | Turns: {} | Hand: Bo1 smoothing + mull\n", results.num_games, results.max_turns);
    println!("{:<6} {:>8} {:>8} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8}",
        "Turn", "AvgMana", "Median", "P25", "P75", "P90", "Max", "AvgLand", "AvgCrt");
    println!("{}", "-".repeat(76));

    for t in 0..results.max_turns {
        let mut mana = results.turn_mana_values[t].clone();
        mana.sort();
        let n = mana.len() as f64;
        let avg_mana: f64 = mana.iter().sum::<usize>() as f64 / n;
        let median = percentile(&mana, 50.0);
        let p25 = percentile(&mana, 25.0);
        let p75 = percentile(&mana, 75.0);
        let p90 = percentile(&mana, 90.0);
        let max_val = mana.last().copied().unwrap_or(0);
        let avg_land: f64 = results.turn_land_values[t].iter().sum::<usize>() as f64 / n;
        let avg_crt: f64 = results.turn_creature_values[t].iter().sum::<usize>() as f64 / n;
        println!("{:<6} {:>8.2} {:>8} {:>6} {:>6} {:>6} {:>6} {:>8.2} {:>8.2}",
            t + 1, avg_mana, median, p25, p75, p90, max_val, avg_land, avg_crt);
    }

    let dork_pct = results.mana_dork_turn_1_count as f64 / results.num_games as f64 * 100.0;
    println!("\nTurn-1 mana dork: {:.1}% ({}/{})",
        dork_pct, results.mana_dork_turn_1_count, results.num_games);
}