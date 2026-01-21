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

    /// Optimize land configuration
    Optimize {
        /// Number of random configurations to test
        #[arg(short, long, default_value = "100")]
        configs: usize,

        /// Number of games per configuration
        #[arg(short, long, default_value = "1000")]
        games: usize,

        /// Strategy for generating land configurations: "weighted" or "shuffle"
        #[arg(short, long, default_value = "weighted")]
        strategy: String,

        /// Base deck file to use for fixed cards (lands will be replaced)
        #[arg(short, long, default_value = "deck.txt")]
        deck: String,
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
        Some(Commands::Optimize { configs, games, strategy, deck }) => {
            optimize_lands(&db, configs, games, &strategy, &deck);
        }
        Some(Commands::Analyze { num_games, deck, seed }) => {
            analyze_turn4_failures(&db, &deck, num_games, seed);
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

    // Mana color availability
    let has_ubg: Vec<_> = results.iter().filter(|r| r.turn_with_ubg.is_some()).collect();
    let avg_ubg_turn = if !has_ubg.is_empty() {
        has_ubg
            .iter()
            .map(|r| r.turn_with_ubg.unwrap() as f64)
            .sum::<f64>()
            / has_ubg.len() as f64
    } else {
        0.0
    };

    println!("=== Results ===\n");
    println!("Win rate: {:.1}% ({}/{})", win_rate * 100.0, wins.len(), num_games);
    println!("Average win turn: {:.2}", avg_win_turn);
    println!("Average UBG available: turn {:.2}", avg_ubg_turn);
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

fn optimize_lands(db: &CardDatabase, num_configs: usize, games_per_config: usize, strategy: &str, deck_file: &str) {
    use simulation::optimize::{generate_random_land_config_weighted, generate_random_land_config_shuffle, build_deck_from_config_with_fixed, config_to_string, save_deck_to_file, DeckSaveParams, extract_fixed_cards_from_deck};
    use crate::rng::GameRng;

    let strategy_desc = match strategy {
        "weighted" => "Random counts for each land type, respecting max limits",
        "shuffle" => "Pool of max copies shuffled, take first 24",
        _ => {
            eprintln!("Unknown strategy '{}'. Use 'weighted' or 'shuffle'.", strategy);
            return;
        }
    };

    // Extract fixed (non-land) cards from the deck file
    let fixed_cards = match extract_fixed_cards_from_deck(deck_file, db) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to parse deck file '{}': {}", deck_file, e);
            return;
        }
    };

    let fixed_card_count: usize = fixed_cards.iter().map(|(_, count)| count).sum();

    println!("\n=== MTG Land Optimization ===\n");
    println!("Base deck: {}", deck_file);
    println!("Strategy: {}", strategy);
    println!("  - {}\n", strategy_desc);
    println!("Testing {} random land configurations", num_configs);
    println!("Running {} games per configuration...\n", games_per_config);
    println!("Fixed non-land cards: {} cards", fixed_card_count);
    println!("Land slots to fill: 24 cards\n");

    let mut best_config = None;
    let mut best_avg_turn = f64::INFINITY;
    let mut best_win_rate = 0.0;
    let mut best_turn_distribution: HashMap<u32, usize> = HashMap::new();
    let mut all_results: Vec<(simulation::optimize::LandConfig, f64, f64)> = Vec::new();

    let start = std::time::Instant::now();

    for i in 0..num_configs {
        // Generate random land configuration using selected strategy
        let mut rng = GameRng::new(None);
        let config = match strategy {
            "shuffle" => generate_random_land_config_shuffle(&mut rng),
            _ => generate_random_land_config_weighted(&mut rng),
        };

        // Build deck from config using the fixed cards from the deck file
        let deck = match build_deck_from_config_with_fixed(&config, &fixed_cards, db) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error building deck: {}", e);
                continue;
            }
        };

        // Run games with this configuration
        let deck_results: Vec<_> = (0..games_per_config)
            .into_par_iter()
            .map(|j| {
                let seed = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64)
                    .wrapping_add(j as u64);
                run_game(&deck, seed, db, false)
            })
            .collect();

        let wins: Vec<_> = deck_results.iter().filter(|r| r.win_turn.is_some()).collect();
        let win_rate = wins.len() as f64 / games_per_config as f64;
        let avg_win_turn = if !wins.is_empty() {
            wins.iter().map(|r| r.win_turn.unwrap() as f64).sum::<f64>() / wins.len() as f64
        } else {
            f64::INFINITY
        };

        all_results.push((config.clone(), win_rate, avg_win_turn));

        // Track best configuration
        if avg_win_turn > 0.0 && avg_win_turn < best_avg_turn {
            best_config = Some(config.clone());
            best_avg_turn = avg_win_turn;
            best_win_rate = win_rate;

            // Build turn distribution for the new best config
            best_turn_distribution.clear();
            for result in &wins {
                if let Some(turn) = result.win_turn {
                    *best_turn_distribution.entry(turn).or_insert(0) += 1;
                }
            }

            println!("[{}/{}] New best! Avg turn: {:.3}, Win rate: {:.1}%",
                i + 1, num_configs, best_avg_turn, best_win_rate * 100.0);
            println!("  Lands: {}\n", config_to_string(&config));
        }

        // Progress update every 100 configs
        if (i + 1) % 100 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let eta = (elapsed / (i + 1) as f64) * (num_configs - i - 1) as f64;
            println!("Progress: {}/{} ({:.1}%) - ETA: {:.0}s",
                i + 1, num_configs, (i + 1) as f64 / num_configs as f64 * 100.0, eta);
        }
    }

    let total_time = start.elapsed().as_secs_f64();

    println!("\n=== Optimization Complete ===");
    println!("Total time: {:.1}s", total_time);
    println!("Configurations tested: {}", num_configs);
    println!("Games per config: {}", games_per_config);
    println!("Total games: {}\n", num_configs * games_per_config);

    println!("=== BEST LAND CONFIGURATION ===");
    println!("Average win turn: {:.3}", best_avg_turn);
    println!("Win rate: {:.1}%", best_win_rate * 100.0);
    println!("\nLand breakdown:");
    if let Some(config) = &best_config {
        let mut lands: Vec<_> = config.iter().filter(|(_, count)| **count > 0).collect();
        lands.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for (name, count) in lands {
            println!("  {} {}", count, name);
        }
    }

    // Show top 10 configurations
    println!("\n=== Top 10 Configurations ===");
    all_results.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    for (i, (config, win_rate, avg_turn)) in all_results.iter().take(10).enumerate() {
        println!("[{}] Avg turn: {:.3}, Win rate: {:.1}%", i + 1, avg_turn, win_rate * 100.0);
        println!("    {}", config_to_string(config));
    }

    // Save best deck to file with all optimization metadata
    if let Some(config) = &best_config {
        let params = DeckSaveParams {
            win_rate: best_win_rate,
            avg_win_turn: best_avg_turn,
            num_simulations: games_per_config,
            strategy: strategy.to_string(),
            turn_distribution: best_turn_distribution,
            fixed_cards: &fixed_cards,
        };
        match save_deck_to_file(config, &params) {
            Ok(filename) => println!("\nBest deck saved to: {}", filename),
            Err(e) => eprintln!("\nFailed to save deck: {}", e),
        }
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

        if **reason == FailureReason::ComboAvailable {
            println!("  {:30} {:5.1}% {} ({})",
                format!("{}", reason), pct, bar, count);
        } else {
            println!("  {:30} {:5.1}% {} ({})",
                format!("{}", reason), pct, bar, count);
        }
    }

    println!("\n--- Statistics ---\n");
    println!("Average lands by turn 4: {:.2}", results.avg_lands);
    println!("Color availability:");
    println!("  Blue:  {:5.1}%", results.color_availability.0);
    println!("  Black: {:5.1}%", results.color_availability.1);
    println!("  Green: {:5.1}%", results.color_availability.2);

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
