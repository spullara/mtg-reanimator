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
use std::sync::atomic::{AtomicUsize, Ordering};

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
        Some(Commands::Optimize { configs, games }) => {
            optimize_lands(&db, configs, games);
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

fn optimize_lands(db: &CardDatabase, num_configs: usize, games_per_config: usize) {
    println!("\n=== MTG Land Optimization ===\n");
    println!("Testing {} random land configurations", num_configs);
    println!("Running {} games per configuration", games_per_config);
    println!();
    println!("Note: Full optimization requires implementing deck mutation.");
    println!("This is a placeholder for the optimization framework.");
    println!();

    // For now, just test the existing deck files
    let deck_files = ["deck.txt", "deck2.txt", "deck3.txt", "deck4.txt", "deck5.txt"];
    let mut results: Vec<(&str, f64, f64)> = Vec::new();

    let progress = AtomicUsize::new(0);
    let total = deck_files.len();

    for deck_file in &deck_files {
        if let Ok(deck) = parse_deck_file(deck_file, db) {
            let deck_results: Vec<_> = (0..games_per_config)
                .into_par_iter()
                .map(|i| {
                    let seed = (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64)
                        .wrapping_add(i as u64);
                    run_game(&deck, seed, db, false)
                })
                .collect();

            let wins: Vec<_> = deck_results.iter().filter(|r| r.win_turn.is_some()).collect();
            let win_rate = wins.len() as f64 / games_per_config as f64;
            let avg_win = if !wins.is_empty() {
                wins.iter().map(|r| r.win_turn.unwrap() as f64).sum::<f64>() / wins.len() as f64
            } else {
                0.0
            };

            results.push((deck_file, win_rate, avg_win));
            let done = progress.fetch_add(1, Ordering::Relaxed) + 1;
            println!("  [{}/{}] {} - {:.1}% win rate, avg turn {:.2}", done, total, deck_file, win_rate * 100.0, avg_win);
        }
    }

    println!("\n=== Best Deck Configurations ===\n");
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (i, (deck, win_rate, avg_turn)) in results.iter().enumerate() {
        let marker = if i == 0 { "★" } else { " " };
        println!(
            "{} {:15} - Win rate: {:5.1}%, Avg turn: {:.2}",
            marker,
            deck,
            win_rate * 100.0,
            avg_turn
        );
    }
}
