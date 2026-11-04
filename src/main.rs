mod card;
mod cli;
mod game;
mod rng;
mod simulation;

use card::CardDatabase;
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use simulation::deck::parse_deck_file;
use simulation::engine::run_game;
use std::collections::HashMap;


#[derive(Parser)]
#[command(name = "mtg-reanimator")]
#[command(about = "MTG Reanimator Combo Deck Simulator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Seed for random number generator (for reproducibility)
    #[arg(short, long)]
    seed: Option<u64>,

    /// Deck file to use
    #[arg(short, long, default_value = "deck.txt")]
    deck: String,

    /// Enable verbose output for single game
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single game or batch of games (default)
    Run {
        /// Number of games to simulate
        #[arg(short, long, default_value = "1000")]
        num_games: usize,

        /// Deck file to use
        #[arg(short, long, default_value = "deck.txt")]
        deck: String,

        /// Seed for reproducibility
        #[arg(short, long)]
        seed: Option<u64>,

        /// Enable verbose output for single game
        #[arg(short, long)]
        verbose: bool,
    },

    /// Compare two deck configurations
    Compare {
        /// First deck file
        deck1: String,

        /// Second deck file
        deck2: String,

        /// Number of games per deck
        #[arg(short, long, default_value = "1000")]
        num_games: usize,
    },

    /// Optimize land configuration (3 stages: random search, top-10 retest, local search)
    Optimize {
        /// Number of random configurations to test in stage 1
        #[arg(short, long, default_value = "200")]
        configs: usize,

        /// Number of games per configuration in stage 1
        #[arg(short, long, default_value = "2000")]
        games: usize,

        /// Base deck file to use for fixed cards (lands will be replaced)
        #[arg(short, long, default_value = "deck.txt")]
        deck: String,

        /// Land pool file: lines '<count-min> <count-max> <Land name>'.
        /// Without this flag, the built-in pool for the old U/B/G deck is used.
        #[arg(short, long)]
        pool: Option<String>,

        /// Base seed. Game i of every configuration uses seed base+i,
        /// so all configurations play the same shuffles and results repeat.
        #[arg(short, long, default_value = "1")]
        seed: u64,

        /// Output deck file names, comma-separated, one per land count
        /// in the same order as --land-counts (default: deck_<hash>.txt)
        #[arg(short, long)]
        out: Option<String>,

        /// Land counts to optimize, comma-separated. Allowed: 23, 24, 25.
        /// 25 cuts 1 spell from the base deck; 23 adds 1 copy of a spell
        /// that is already in the deck. Both start from the best 24-land
        /// configuration.
        #[arg(short, long, default_value = "24")]
        land_counts: String,
    },

    /// Analyze turn 4 combo failure reasons
    Analyze {
        /// Number of games to simulate
        #[arg(short, long, default_value = "1000")]
        num_games: usize,

        /// Deck file to use
        #[arg(short, long, default_value = "deck.txt")]
        deck: String,

        /// Seed for reproducibility
        #[arg(short, long)]
        seed: Option<u64>,
    },

    /// Simulate mana production per turn
    Mana {
        /// Deck file to use
        #[arg(short, long, default_value = "deck.txt")]
        deck: String,

        /// Number of games to simulate
        #[arg(short, long, default_value = "10000")]
        num_games: usize,

        /// Number of turns to simulate
        #[arg(short, long, default_value = "10")]
        turns: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    // Load the card database
    let db = match CardDatabase::from_file("cards.json") {
        Ok(db) => {
            eprintln!("✓ Loaded {} cards from cards.json", db.card_count());
            db
        }
        Err(e) => {
            eprintln!("✗ Failed to load cards: {}", e);
            std::process::exit(1);
        }
    };

    match cli.command {
        Some(Commands::Run {
            num_games,
            deck,
            seed,
            verbose,
        }) => {
            run_simulation(&db, &deck, num_games, seed, verbose);
        }
        Some(Commands::Compare {
            deck1,
            deck2,
            num_games,
        }) => {
            compare_decks(&db, &deck1, &deck2, num_games);
        }
        Some(Commands::Optimize { configs, games, deck, pool, seed, out, land_counts }) => {
            optimize_lands(&db, configs, games, &deck, pool.as_deref(), seed, out, &land_counts);
        }
        Some(Commands::Analyze { num_games, deck, seed }) => {
            analyze_turn4_failures(&db, &deck, num_games, seed);
        }
        Some(Commands::Mana { deck, num_games, turns }) => {
            run_mana_sim(&db, &deck, num_games, turns);
        }
        None => {
            // Default: run simulation with CLI args
            let num_games = if cli.verbose { 1 } else { 1000 };
            run_simulation(&db, &cli.deck, num_games, cli.seed, cli.verbose);
        }
    }
}

fn run_simulation(db: &CardDatabase, deck_file: &str, num_games: usize, seed: Option<u64>, verbose: bool) {
    let deck = match parse_deck_file(deck_file, db) {
        Ok(deck) => deck,
        Err(e) => {
            eprintln!("✗ Failed to parse deck file '{}': {}", deck_file, e);
            std::process::exit(1);
        }
    };

    println!("\n=== MTG Reanimator Simulator ===\n");
    println!("Deck: {} ({} cards)", deck_file, deck.len());
    println!("Games: {}", num_games);
    if let Some(s) = seed {
        println!("Seed: {}", s);
    }
    println!();

    let start = std::time::Instant::now();
    let results: Vec<_> = if let Some(base_seed) = seed {
        // Sequential with fixed seed
        (0..num_games)
            .map(|i| run_game(&deck, base_seed + i as u64, db, verbose && i == 0))
            .collect()
    } else if verbose {
        // Sequential for verbose mode (verbose only makes sense for first game)
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        println!("Seed: {}", seed);
        (0..num_games)
            .map(|i| run_game(&deck, seed.wrapping_add(i as u64), db, i == 0))
            .collect()
    } else {
        // Parallel with random seeds
        (0..num_games)
            .into_par_iter()
            .map(|i| {
                let seed = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64)
                    .wrapping_add(i as u64);
                run_game(&deck, seed, db, false)
            })
            .collect()
    };
    let elapsed = start.elapsed();

    // Calculate statistics
    let wins: Vec<_> = results.iter().filter(|r| r.win_turn.is_some()).collect();
    let win_rate = wins.len() as f64 / num_games as f64;

    let avg_win_turn = if !wins.is_empty() {
        wins.iter().map(|r| r.win_turn.unwrap() as f64).sum::<f64>() / wins.len() as f64
    } else {
        0.0
    };

    // Turn distribution
    let mut turn_dist: HashMap<u32, usize> = HashMap::new();
    for r in &results {
        if let Some(turn) = r.win_turn {
            *turn_dist.entry(turn).or_insert(0) += 1;
        }
    }

    // Turn 4 mana check (main phase, after the land drop, before spending mana)
    let games_with_4plus = results.iter()
        .filter(|r| r.turn4_mana.map_or(false, |m| m.had_4plus_mana))
        .count();
    let wrong_mana_games = results.iter()
        .filter(|r| r.turn4_mana.map_or(false, |m| m.wrong_mana))
        .count();
    let wrong_mana_pct_games = wrong_mana_games as f64 / num_games as f64 * 100.0;
    let wrong_mana_pct_4plus = if games_with_4plus > 0 {
        wrong_mana_games as f64 / games_with_4plus as f64 * 100.0
    } else {
        0.0
    };

    // First turn on which {2}{U}{B} could be paid
    let can_pay: Vec<u32> = results.iter()
        .filter_map(|r| r.first_turn_can_pay_spider_man)
        .collect();
    let avg_first_pay_turn = if !can_pay.is_empty() {
        can_pay.iter().map(|t| *t as f64).sum::<f64>() / can_pay.len() as f64
    } else {
        0.0
    };

    println!("=== Results ===\n");
    println!("Win rate: {:.1}% ({}/{})", win_rate * 100.0, wins.len(), num_games);
    println!("Average win turn: {:.2}", avg_win_turn);
    println!(
        "Wrong mana on turn 4: {:.1}% of games ({:.1}% of games with 4+ mana)",
        wrong_mana_pct_games, wrong_mana_pct_4plus
    );
    println!(
        "First turn able to pay {{2}}{{U}}{{B}}: {:.2} on average ({:.1}% of games)",
        avg_first_pay_turn,
        can_pay.len() as f64 / num_games as f64 * 100.0
    );
    println!();

    println!("Turn distribution:");
    let mut turns: Vec<_> = turn_dist.iter().collect();
    turns.sort_by_key(|(t, _)| *t);
    for (turn, count) in turns {
        let pct = *count as f64 / num_games as f64 * 100.0;
        let bar = "█".repeat((pct / 2.0) as usize);
        println!("  Turn {:2}: {:5.1}% {} ({})", turn, pct, bar, count);
    }

    let no_win = results.iter().filter(|r| r.win_turn.is_none()).count();
    if no_win > 0 {
        let pct = no_win as f64 / num_games as f64 * 100.0;
        println!("  No win: {:5.1}% ({})", pct, no_win);
    }

    println!();
    println!(
        "Simulation completed in {:.2?} ({:.0} games/sec)",
        elapsed,
        num_games as f64 / elapsed.as_secs_f64()
    );
}

fn compare_decks(db: &CardDatabase, deck1_file: &str, deck2_file: &str, num_games: usize) {
    println!("\n=== MTG Deck Comparison ===\n");
    println!("Deck 1: {}", deck1_file);
    println!("Deck 2: {}", deck2_file);
    println!("Games per deck: {}", num_games);
    println!();

    let deck1 = match parse_deck_file(deck1_file, db) {
        Ok(deck) => deck,
        Err(e) => {
            eprintln!("✗ Failed to parse deck1 '{}': {}", deck1_file, e);
            std::process::exit(1);
        }
    };

    let deck2 = match parse_deck_file(deck2_file, db) {
        Ok(deck) => deck,
        Err(e) => {
            eprintln!("✗ Failed to parse deck2 '{}': {}", deck2_file, e);
            std::process::exit(1);
        }
    };

    let start = std::time::Instant::now();

    // Run deck 1
    println!("Running deck 1...");
    let results1: Vec<_> = (0..num_games)
        .into_par_iter()
        .map(|i| {
            let seed = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64)
                .wrapping_add(i as u64);
            run_game(&deck1, seed, db, false)
        })
        .collect();

    // Run deck 2
    println!("Running deck 2...");
    let results2: Vec<_> = (0..num_games)
        .into_par_iter()
        .map(|i| {
            let seed = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64)
                .wrapping_add(i as u64 + num_games as u64);
            run_game(&deck2, seed, db, false)
        })
        .collect();

    let elapsed = start.elapsed();

    // Calculate stats for deck 1
    let wins1: Vec<_> = results1.iter().filter(|r| r.win_turn.is_some()).collect();
    let win_rate1 = wins1.len() as f64 / num_games as f64;
    let avg_win1 = if !wins1.is_empty() {
        wins1.iter().map(|r| r.win_turn.unwrap() as f64).sum::<f64>() / wins1.len() as f64
    } else {
        0.0
    };

    // Calculate stats for deck 2
    let wins2: Vec<_> = results2.iter().filter(|r| r.win_turn.is_some()).collect();
    let win_rate2 = wins2.len() as f64 / num_games as f64;
    let avg_win2 = if !wins2.is_empty() {
        wins2.iter().map(|r| r.win_turn.unwrap() as f64).sum::<f64>() / wins2.len() as f64
    } else {
        0.0
    };

    println!("\n=== Results ===\n");
    println!(
        "{:20} {:>12} {:>12}",
        "Metric", deck1_file, deck2_file
    );
    println!("{:-<50}", "");
    println!(
        "{:20} {:>11.1}% {:>11.1}%",
        "Win rate",
        win_rate1 * 100.0,
        win_rate2 * 100.0
    );
    println!(
        "{:20} {:>12.2} {:>12.2}",
        "Avg win turn", avg_win1, avg_win2
    );

    // Determine winner
    println!();
    if win_rate1 > win_rate2 {
        println!(
            "✓ {} has {:.1}% higher win rate",
            deck1_file,
            (win_rate1 - win_rate2) * 100.0
        );
    } else if win_rate2 > win_rate1 {
        println!(
            "✓ {} has {:.1}% higher win rate",
            deck2_file,
            (win_rate2 - win_rate1) * 100.0
        );
    } else {
        println!("Both decks have the same win rate");
    }

    if avg_win1 < avg_win2 && avg_win1 > 0.0 {
        println!(
            "✓ {} wins {:.2} turns faster on average",
            deck1_file,
            avg_win2 - avg_win1
        );
    } else if avg_win2 < avg_win1 && avg_win2 > 0.0 {
        println!(
            "✓ {} wins {:.2} turns faster on average",
            deck2_file,
            avg_win1 - avg_win2
        );
    }

    println!("\nCompleted in {:.2?}", elapsed);
}

fn optimize_lands(
    db: &CardDatabase,
    num_configs: usize,
    games_per_config: usize,
    deck_file: &str,
    pool_file: Option<&str>,
    base_seed: u64,
    out_arg: Option<String>,
    land_counts_arg: &str,
) {
    use simulation::optimize::{
        default_land_types, parse_pool_file, validate_pool_total, generate_random_land_config,
        evaluate_config, neighbor_swaps, config_to_string, calculate_deck_hash_with_fixed,
        parse_set_codes, save_deck_to_file, DeckSaveParams, extract_fixed_cards_from_deck,
        EvalResult, FixedCards, LandConfig, DEFAULT_TOTAL_LANDS,
    };
    use crate::rng::GameRng;

    const STAGE2_GAMES: usize = 50_000;
    const STAGE3_GAMES: usize = 20_000;
    const FINAL_GAMES: usize = 50_000;
    const STAGE2_TOP: usize = 10;
    const IMPROVE_THRESHOLD: f64 = 0.01;
    const NEVER_CUT: [&str; 2] = ["Superior Spider-Man", "Bringer of the Last Gift"];

    // Parse --land-counts
    let mut land_counts: Vec<usize> = Vec::new();
    for part in land_counts_arg.split(',') {
        let part = part.trim();
        match part.parse::<usize>() {
            Ok(n @ 23..=25) => {
                if !land_counts.contains(&n) {
                    land_counts.push(n);
                }
            }
            _ => {
                eprintln!("✗ Invalid land count '{}'. Allowed: 23, 24, 25.", part);
                return;
            }
        }
    }

    // Parse --out: one file name per land count, same order
    let mut out_names: HashMap<usize, String> = HashMap::new();
    if let Some(out) = &out_arg {
        let outs: Vec<&str> = out.split(',').map(|s| s.trim()).collect();
        if outs.len() != land_counts.len() {
            eprintln!("✗ --out has {} names but --land-counts has {} counts.", outs.len(), land_counts.len());
            return;
        }
        for (count, name) in land_counts.iter().zip(outs) {
            out_names.insert(*count, name.to_string());
        }
    }

    // Land pool: from --pool file, or the built-in default pool
    let pool = match pool_file {
        Some(path) => match parse_pool_file(path) {
            Ok(pool) => pool,
            Err(e) => {
                eprintln!("✗ {}", e);
                return;
            }
        },
        None => default_land_types(),
    };
    for count in land_counts.iter().chain([DEFAULT_TOTAL_LANDS].iter()) {
        if let Err(e) = validate_pool_total(&pool, *count) {
            eprintln!("✗ {}", e);
            return;
        }
    }

    // Extract fixed (non-land) cards from the deck file
    let base_fixed = match extract_fixed_cards_from_deck(deck_file, db) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to parse deck file '{}': {}", deck_file, e);
            return;
        }
    };
    let fixed_card_count: usize = base_fixed.iter().map(|(_, count)| count).sum();

    println!("\n=== MTG Land Optimization (3 stages) ===\n");
    println!("Base deck: {} ({} fixed non-land cards + {} land slots)", deck_file, fixed_card_count, DEFAULT_TOTAL_LANDS);
    match pool_file {
        Some(path) => println!("Land pool: {} ({} land types)", path, pool.len()),
        None => println!("Land pool: built-in default ({} land types)", pool.len()),
    }
    println!("Land counts: {:?}", land_counts);
    println!("Seed base: {} (game i uses seed {}+i for every configuration)", base_seed, base_seed);

    let eval = |config: &LandConfig, fixed: &FixedCards, games: usize| -> Option<EvalResult> {
        match evaluate_config(config, fixed, db, games, base_seed) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("Error evaluating config: {}", e);
                None
            }
        }
    };

    // Local search: try every single swap at STAGE3_GAMES, move to the best
    // swap while it is at least IMPROVE_THRESHOLD turns better.
    // Returns the final config, its STAGE3_GAMES eval, and the swap results
    // of the last (stopping) iteration.
    type SwapEval = (String, String, EvalResult);
    let local_search = |start: LandConfig, start_eval: EvalResult, fixed: &FixedCards|
        -> (LandConfig, EvalResult, Vec<SwapEval>) {
        let mut current = start;
        let mut current_eval = start_eval;
        let mut last_swaps: Vec<SwapEval>;
        loop {
            let swaps = neighbor_swaps(&current, &pool);
            println!("Testing {} single swaps...", swaps.len());
            let mut results: Vec<SwapEval> = Vec::new();
            for (donor, receiver, next) in swaps {
                if let Some(result) = eval(&next, fixed, STAGE3_GAMES) {
                    results.push((donor, receiver, result));
                }
            }
            let best = results
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.2.score_cmp(&b.2))
                .map(|(i, _)| i);
            last_swaps = results;
            match best {
                Some(i) if last_swaps[i].2.avg_win_turn <= current_eval.avg_win_turn - IMPROVE_THRESHOLD => {
                    let (donor, receiver, result) = last_swaps[i].clone();
                    println!("Move: -1 {}, +1 {} => avg turn {:.3} (was {:.3})",
                        donor, receiver, result.avg_win_turn, current_eval.avg_win_turn);
                    let mut next = current.clone();
                    *next.get_mut(&donor).unwrap() -= 1;
                    *next.entry(receiver.clone()).or_insert(0) += 1;
                    next.retain(|_, c| *c > 0);
                    current = next;
                    current_eval = result;
                }
                Some(i) => {
                    let (donor, receiver, result) = &last_swaps[i];
                    println!("Best swap (-1 {}, +1 {}) gives {:.3}, less than {:.2} better than {:.3}. Stop.",
                        donor, receiver, result.avg_win_turn, IMPROVE_THRESHOLD, current_eval.avg_win_turn);
                    break;
                }
                None => {
                    println!("No legal swaps. Stop.");
                    break;
                }
            }
        }
        (current, current_eval, last_swaps)
    };

    let save = |config: &LandConfig, fixed: &FixedCards, final_eval: &EvalResult,
                count: usize, change_note: Option<String>| {
        let set_codes = parse_set_codes(deck_file);
        let params = DeckSaveParams {
            eval: final_eval,
            base_seed,
            strategy: "3-stage: random search, top-10 retest, local search".to_string(),
            fixed_cards: fixed,
            set_codes: &set_codes,
            out_file: out_names.get(&count).cloned(),
            change_note,
        };
        match save_deck_to_file(config, &params) {
            Ok(filename) => println!("\n{}-land deck saved to: {}", count, filename),
            Err(e) => eprintln!("\nFailed to save {}-land deck: {}", count, e),
        }
    };

    let print_final = |label: &str, final_eval: &EvalResult, config: &LandConfig| {
        println!("\n=== BEST {} CONFIGURATION ===", label);
        println!("Average win turn: {:.3}", final_eval.avg_win_turn);
        println!("Win rate: {:.1}%", final_eval.win_rate * 100.0);
        println!("Turn 4 win: {:.2}%  Turn 5 win: {:.2}%  Turn 6 win: {:.2}%",
            final_eval.turn_pct(4), final_eval.turn_pct(5), final_eval.turn_pct(6));
        println!("Wrong mana on turn 4: {:.2}%", final_eval.wrong_mana_turn4_pct);
        println!("Land breakdown:");
        let mut lands: Vec<_> = config.iter().filter(|(_, count)| **count > 0).collect();
        lands.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for (name, count) in lands {
            println!("  {} {}", count, name);
        }
    };

    let start = std::time::Instant::now();

    // --- Stage 1: random search at 24 lands ---
    println!("\n--- Stage 1 (24 lands): {} random configurations at {} games each ---\n", num_configs, games_per_config);
    let mut config_rng = GameRng::new(Some(base_seed));
    let mut stage1: Vec<(LandConfig, EvalResult)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for i in 0..num_configs {
        let config = generate_random_land_config(&pool, &mut config_rng, DEFAULT_TOTAL_LANDS);
        if !seen.insert(calculate_deck_hash_with_fixed(&config, &base_fixed)) {
            continue; // duplicate configuration, skip
        }
        let Some(result) = eval(&config, &base_fixed, games_per_config) else { continue };
        stage1.push((config, result));

        if (i + 1) % 20 == 0 || i + 1 == num_configs {
            let best = stage1
                .iter()
                .min_by(|a, b| a.1.score_cmp(&b.1))
                .map(|(_, r)| r.avg_win_turn)
                .unwrap_or(f64::INFINITY);
            let elapsed = start.elapsed().as_secs_f64();
            let eta = (elapsed / (i + 1) as f64) * (num_configs - i - 1) as f64;
            println!("[{}/{}] best avg turn so far: {:.3} (elapsed {:.0}s, ETA {:.0}s)",
                i + 1, num_configs, best, elapsed, eta);
        }
    }

    if stage1.is_empty() {
        eprintln!("No configurations could be evaluated.");
        return;
    }

    stage1.sort_by(|a, b| a.1.score_cmp(&b.1));

    // --- Stage 2: re-test the top configurations ---
    let top_n = stage1.len().min(STAGE2_TOP);
    println!("\n--- Stage 2 (24 lands): re-test top {} at {} games each ---\n", top_n, STAGE2_GAMES);
    let mut stage2: Vec<(LandConfig, EvalResult)> = Vec::new();
    for (i, (config, s1)) in stage1.iter().take(top_n).enumerate() {
        let Some(result) = eval(config, &base_fixed, STAGE2_GAMES) else { continue };
        println!("[{}/{}] avg turn {:.3} (stage 1: {:.3}), T4+T5 {:.1}%  |  {}",
            i + 1, top_n, result.avg_win_turn, s1.avg_win_turn, result.t45_pct(), config_to_string(config));
        stage2.push((config.clone(), result));
    }
    stage2.sort_by(|a, b| a.1.score_cmp(&b.1));

    // --- Stage 3 (24 lands): local search ---
    println!("\n--- Stage 3 (24 lands): local search from the best configuration ({} games per swap) ---", STAGE3_GAMES);
    let stage3_start = stage2[0].0.clone();
    let Some(stage3_start_eval) = eval(&stage3_start, &base_fixed, STAGE3_GAMES) else { return };
    println!("\nStart: avg turn {:.3}  |  {}", stage3_start_eval.avg_win_turn, config_to_string(&stage3_start));
    let (best24, _best24_eval, last_swaps24) = local_search(stage3_start, stage3_start_eval, &base_fixed);

    println!("\n--- Final confirmation (24 lands) at {} games ---", FINAL_GAMES);
    let Some(final24) = eval(&best24, &base_fixed, FINAL_GAMES) else { return };
    print_final("24-LAND", &final24, &best24);

    println!("\n=== Top {} stage-2 configurations (24 lands, {} games) ===", stage2.len(), STAGE2_GAMES);
    for (i, (config, result)) in stage2.iter().enumerate() {
        println!("[{}] avg turn {:.3}, T4 {:.2}%, T5 {:.2}%, T6 {:.2}%, wrong mana T4 {:.2}%",
            i + 1, result.avg_win_turn, result.turn_pct(4), result.turn_pct(5),
            result.turn_pct(6), result.wrong_mana_turn4_pct);
        println!("    {}", config_to_string(config));
    }

    if land_counts.contains(&DEFAULT_TOTAL_LANDS) {
        save(&best24, &base_fixed, &final24, DEFAULT_TOTAL_LANDS, None);
    }

    // Land values from the last 24-land local-search iteration:
    // most valued = receiver whose best "+1" swap scored best;
    // least valued = donor whose best "-1" swap scored best (cheapest to give up).
    let mut best_as_receiver: HashMap<&str, f64> = HashMap::new();
    let mut best_as_donor: HashMap<&str, f64> = HashMap::new();
    for (donor, receiver, result) in &last_swaps24 {
        let r = best_as_receiver.entry(receiver).or_insert(f64::INFINITY);
        *r = r.min(result.avg_win_turn);
        let d = best_as_donor.entry(donor).or_insert(f64::INFINITY);
        *d = d.min(result.avg_win_turn);
    }
    let most_valued = best_as_receiver
        .iter()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(name, _)| name.to_string());
    let least_valued = best_as_donor
        .iter()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(name, _)| name.to_string());

    // --- Other land counts: 25 (cut 1 spell) and 23 (add 1 spell) ---
    let mut summary: Vec<(usize, String, EvalResult, LandConfig)> = Vec::new();
    if land_counts.contains(&DEFAULT_TOTAL_LANDS) {
        summary.push((DEFAULT_TOTAL_LANDS, "no spell change".to_string(), final24.clone(), best24.clone()));
    }

    for &count in &land_counts {
        if count == DEFAULT_TOTAL_LANDS {
            continue;
        }

        // Derive the starting land config from the best 24-land config
        let (land_config, land_change) = if count > DEFAULT_TOTAL_LANDS {
            let Some(add) = &most_valued else {
                eprintln!("No swap data to pick the most valued land; skipping {} lands.", count);
                continue;
            };
            let mut c = best24.clone();
            *c.entry(add.clone()).or_insert(0) += 1;
            (c, format!("+1 {}", add))
        } else {
            let Some(remove) = &least_valued else {
                eprintln!("No swap data to pick the least valued land; skipping {} lands.", count);
                continue;
            };
            let mut c = best24.clone();
            *c.get_mut(remove.as_str()).unwrap() -= 1;
            c.retain(|_, n| *n > 0);
            (c, format!("-1 {}", remove))
        };

        // Spell candidates
        let cutting = count > DEFAULT_TOTAL_LANDS;
        let candidates: Vec<String> = if cutting {
            base_fixed.iter()
                .filter(|(name, n)| *n >= 2 && !NEVER_CUT.contains(&name.as_str()))
                .map(|(name, _)| name.clone())
                .collect()
        } else {
            base_fixed.iter()
                .filter(|(_, n)| *n < 4)
                .map(|(name, _)| name.clone())
                .collect()
        };

        println!("\n--- {} lands ({}): {} 1 spell, {} candidates at {} games each ---\n",
            count, land_change, if cutting { "cut" } else { "add" }, candidates.len(), STAGE3_GAMES);

        let mut best_spell: Option<(String, FixedCards, EvalResult)> = None;
        for name in &candidates {
            let fixed: FixedCards = base_fixed.iter()
                .filter_map(|(n, c)| {
                    let c = if n == name { if cutting { c - 1 } else { c + 1 } } else { *c };
                    if c > 0 { Some((n.clone(), c)) } else { None }
                })
                .collect();
            let Some(result) = eval(&land_config, &fixed, STAGE3_GAMES) else { continue };
            println!("  {} 1 {:28} => avg turn {:.3}, T4+T5 {:.1}%",
                if cutting { "cut" } else { "add" }, name, result.avg_win_turn, result.t45_pct());
            let better = match &best_spell {
                None => true,
                Some((_, _, b)) => result.score_cmp(b) == std::cmp::Ordering::Less,
            };
            if better {
                best_spell = Some((name.clone(), fixed, result));
            }
        }

        let Some((spell, fixed, spell_eval)) = best_spell else {
            eprintln!("No spell candidate could be evaluated; skipping {} lands.", count);
            continue;
        };
        let spell_change = format!("{} 1 {}", if cutting { "cut" } else { "added" }, spell);
        println!("\nBest spell change: {} (avg turn {:.3})", spell_change, spell_eval.avg_win_turn);

        println!("\n--- Stage 3 ({} lands): local search ({} games per swap) ---", count, STAGE3_GAMES);
        let (best_config, _, _) = local_search(land_config, spell_eval, &fixed);

        println!("\n--- Final confirmation ({} lands) at {} games ---", count, FINAL_GAMES);
        let Some(final_eval) = eval(&best_config, &fixed, FINAL_GAMES) else { continue };
        let label = format!("{}-LAND ({}, {})", count, land_change, spell_change);
        print_final(&label, &final_eval, &best_config);

        save(&best_config, &fixed, &final_eval, count,
            Some(format!("{}, {} (vs the base deck)", land_change, spell_change)));
        summary.push((count, spell_change, final_eval, best_config));
    }

    let total_time = start.elapsed().as_secs_f64();
    println!("\n=== Optimization Complete ({:.0}s) ===\n", total_time);
    println!("=== Summary ({} games, seed base {}) ===", FINAL_GAMES, base_seed);
    summary.sort_by_key(|(count, ..)| *count);
    for (count, change, result, config) in &summary {
        println!("[{} lands] avg turn {:.3}, T4 {:.2}%, T5 {:.2}%, T6 {:.2}%, wrong mana T4 {:.2}%  ({})",
            count, result.avg_win_turn, result.turn_pct(4), result.turn_pct(5),
            result.turn_pct(6), result.wrong_mana_turn4_pct, change);
        println!("    {}", config_to_string(config));
    }
}
fn analyze_turn4_failures(db: &CardDatabase, deck_file: &str, num_games: usize, seed: Option<u64>) {
    use simulation::analyze::{run_game_to_turn4, aggregate_results, FailureReason};

    let deck = match parse_deck_file(deck_file, db) {
        Ok(deck) => deck,
        Err(e) => {
            eprintln!("✗ Failed to parse deck file '{}': {}", deck_file, e);
            std::process::exit(1);
        }
    };

    println!("\n=== Turn 4 Combo Failure Analysis ===\n");
    println!("Deck: {} ({} cards)", deck_file, deck.len());
    println!("Games: {}", num_games);
    if let Some(s) = seed {
        println!("Seed: {}", s);
    }
    println!();

    let start = std::time::Instant::now();

    // Run games in parallel
    let analyses: Vec<_> = if let Some(base_seed) = seed {
        (0..num_games)
            .into_par_iter()
            .map(|i| run_game_to_turn4(&deck, base_seed + i as u64, db))
            .collect()
    } else {
        (0..num_games)
            .into_par_iter()
            .map(|i| {
                let seed = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64)
                    .wrapping_add(i as u64);
                run_game_to_turn4(&deck, seed, db)
            })
            .collect()
    };

    let elapsed = start.elapsed();

    // Aggregate results
    let results = aggregate_results(&analyses);

    println!("=== Results ===\n");

    // Sort failures by count (descending)
    let mut failures: Vec<_> = results.failure_counts.iter().collect();
    failures.sort_by(|a, b| b.1.cmp(a.1));

    // Print ranked failure reasons
    println!("Failure Reasons (ranked by frequency):\n");
    for (reason, count) in &failures {
        let pct = **count as f64 / num_games as f64 * 100.0;
        let bar = "█".repeat((pct / 2.0) as usize);
        println!("  {:42} {:5.1}% {} ({})",
            format!("{}", reason), pct, bar, count);
    }

    println!("\n--- Statistics ---\n");
    println!("Average mana on turn 4 (after land drop): {:.2}", results.avg_mana);
    println!("Color availability:");
    println!("  Blue:  {:5.1}%", results.color_availability.0);
    println!("  Black: {:5.1}%", results.color_availability.1);

    println!("\n--- Wrong mana on turn 4 ---\n");
    let pct_4plus = results.games_with_4plus_mana as f64 / num_games as f64 * 100.0;
    println!("Games with 4+ mana: {:.1}% ({}/{})",
        pct_4plus, results.games_with_4plus_mana, num_games);
    let wrong_pct_games = results.wrong_mana_games as f64 / num_games as f64 * 100.0;
    let wrong_pct_4plus = if results.games_with_4plus_mana > 0 {
        results.wrong_mana_games as f64 / results.games_with_4plus_mana as f64 * 100.0
    } else {
        0.0
    };
    println!("Wrong mana (4+ mana, cannot pay {{2}}{{U}}{{B}}): {:.1}% of games ({:.1}% of games with 4+ mana)",
        wrong_pct_games, wrong_pct_4plus);
    use simulation::analyze::WrongManaKind;
    for kind in [
        WrongManaKind::NoBlue,
        WrongManaKind::NoBlack,
        WrongManaKind::NoBlueNoBlack,
        WrongManaKind::CannotPayTogether,
    ] {
        let count = results.wrong_mana_counts.get(&kind).copied().unwrap_or(0);
        println!("  {:38} {:5.2}% ({})",
            format!("{}", kind), count as f64 / num_games as f64 * 100.0, count);
    }

    println!("\n--- Color-locked spells on turn 4 ---\n");
    println!("Games with a color-locked spell in hand: {:.1}% ({}/{})",
        results.color_locked_games as f64 / num_games as f64 * 100.0,
        results.color_locked_games, num_games);
    if !results.color_locked_cards.is_empty() {
        println!("Top color-locked cards (games where the card was locked):");
        for (name, count) in results.color_locked_cards.iter().take(10) {
            println!("  {:38} {:5.2}% ({})",
                name, *count as f64 / num_games as f64 * 100.0, count);
        }
    }

    // Calculate additional stats from raw analyses
    let combo_ready = failures.iter()
        .find(|(r, _)| **r == FailureReason::ComboAvailable)
        .map(|(_, c)| **c)
        .unwrap_or(0);

    println!("\nTurn 4 combo ready: {:.1}% ({}/{})",
        combo_ready as f64 / num_games as f64 * 100.0, combo_ready, num_games);

    println!("\nCompleted in {:.2?} ({:.0} games/sec)",
        elapsed, num_games as f64 / elapsed.as_secs_f64());
}


fn run_mana_sim(db: &CardDatabase, deck_file: &str, num_games: usize, max_turns: usize) {
    use simulation::mana_sim::{run_mana_simulation, print_mana_results};

    let deck = match parse_deck_file(deck_file, db) {
        Ok(deck) => deck,
        Err(e) => {
            eprintln!("✗ Failed to parse deck file '{}': {}", deck_file, e);
            std::process::exit(1);
        }
    };

    let land_count = deck.iter().filter(|c| matches!(c, card::Card::Land(_))).count();
    let deck_size = deck.len();

    let start = std::time::Instant::now();
    let results = run_mana_simulation(&deck, num_games, max_turns, db);
    let elapsed = start.elapsed();

    print_mana_results(&results, deck_file, deck_size, land_count);

    println!("\nCompleted in {:.2?} ({:.0} games/sec)",
        elapsed, num_games as f64 / elapsed.as_secs_f64());
}