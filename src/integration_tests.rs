//! Integration tests for the MTG Reanimator simulator
//! Tests full game simulations with known seeds and validates behavior

use crate::card::CardDatabase;
use crate::simulation::engine::run_game;
use crate::simulation::deck::parse_deck_file;

#[test]
fn test_full_game_with_seed_12345() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    // Run game with known seed
    let result = run_game(&deck, 12345, &db, false);

    // Verify basic properties
    assert!(result.win_turn.is_none() || result.win_turn.unwrap() <= 20, "win_turn should be <= 20");
}

#[test]
fn test_same_seed_produces_same_result() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    // Run same game twice with same seed
    let result1 = run_game(&deck, 54321, &db, false);
    let result2 = run_game(&deck, 54321, &db, false);

    // Results should be identical
    assert_eq!(result1.win_turn, result2.win_turn, "Same seed should produce same win_turn");
    assert_eq!(result1.turn_with_ubg, result2.turn_with_ubg, "Same seed should produce same turn_with_ubg");
}

#[test]
fn test_different_seeds_produce_different_results() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    // Run games with different seeds
    let result1 = run_game(&deck, 111, &db, false);
    let result2 = run_game(&deck, 222, &db, false);

    // At least one property should differ (very unlikely to be identical)
    let results_differ = result1.win_turn != result2.win_turn
        || result1.turn_with_ubg != result2.turn_with_ubg;

    assert!(results_differ, "Different seeds should likely produce different results");
}

#[test]
fn test_game_completes_within_20_turns() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");
    
    // Run multiple games
    for seed in 1..=10 {
        let result = run_game(&deck, seed, &db, false);

        // If game won, it should be within 20 turns
        if let Some(win_turn) = result.win_turn {
            assert!(win_turn <= 20, "Game should win within 20 turns, got turn {}", win_turn);
            assert!(win_turn > 0, "Win turn should be positive");
        }
    }
}

#[test]
fn test_mana_color_tracking() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    let result = run_game(&deck, 99999, &db, false);

    // Verify mana color tracking - if we have UBG, it should be a valid turn
    if let Some(ubg_turn) = result.turn_with_ubg {
        assert!(ubg_turn > 0, "UBG turn should be positive");
        assert!(ubg_turn <= 20, "UBG turn should be within game limit");
    }
}

#[test]
fn test_multiple_deck_files() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");

    // Test all available deck files
    let deck_files = vec!["deck.txt", "deck2.txt", "deck3.txt", "deck4.txt", "deck5.txt"];

    for deck_file in deck_files {
        if let Ok(deck) = parse_deck_file(deck_file, &db) {
            // Each deck should have at least 60 cards (some may have more)
            assert!(deck.len() >= 60, "Deck {} should have at least 60 cards, got {}", deck_file, deck.len());

            // Should be able to run a game
            let _result = run_game(&deck, 42, &db, false);
        }
    }
}

#[test]
fn test_deterministic_rng_sequence() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    // Run 5 games with same seed and verify all results are identical
    let mut results = Vec::new();
    for _ in 0..5 {
        results.push(run_game(&deck, 555, &db, false));
    }

    // All results should be identical
    for i in 1..results.len() {
        assert_eq!(results[0].win_turn, results[i].win_turn);
        assert_eq!(results[0].turn_with_ubg, results[i].turn_with_ubg);
    }
}
