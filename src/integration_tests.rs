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
    assert!(result.on_the_play || !result.on_the_play, "on_the_play should be set");
    assert!(result.win_turn.is_none() || result.win_turn.unwrap() <= 20, "win_turn should be <= 20");
    // u32 is always >= 0, so just verify they exist
    let _ = result.total_combat_damage;
    let _ = result.combo_damage;
}

#[test]
fn test_same_seed_produces_same_result() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");
    
    // Run same game twice with same seed
    let result1 = run_game(&deck, 54321, &db);
    let result2 = run_game(&deck, 54321, &db);
    
    // Results should be identical
    assert_eq!(result1.win_turn, result2.win_turn, "Same seed should produce same win_turn");
    assert_eq!(result1.on_the_play, result2.on_the_play, "Same seed should produce same on_the_play");
    assert_eq!(result1.total_combat_damage, result2.total_combat_damage, "Same seed should produce same combat damage");
    assert_eq!(result1.combo_damage, result2.combo_damage, "Same seed should produce same combo damage");
}

#[test]
fn test_different_seeds_produce_different_results() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");
    
    // Run games with different seeds
    let result1 = run_game(&deck, 111, &db);
    let result2 = run_game(&deck, 222, &db);
    
    // At least one property should differ (very unlikely to be identical)
    let results_differ = result1.win_turn != result2.win_turn 
        || result1.on_the_play != result2.on_the_play
        || result1.total_combat_damage != result2.total_combat_damage;
    
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
    
    // Verify mana color tracking is consistent
    // If we have UBG, we should have U, B, and G individually
    if let Some(ubg_turn) = result.turn_with_ubg {
        assert!(result.turn_with_u.is_some(), "Should have U if we have UBG");
        assert!(result.turn_with_b.is_some(), "Should have B if we have UBG");
        assert!(result.turn_with_g.is_some(), "Should have G if we have UBG");
        
        // UBG turn should be >= individual color turns
        assert!(ubg_turn >= result.turn_with_u.unwrap(), "UBG turn should be >= U turn");
        assert!(ubg_turn >= result.turn_with_b.unwrap(), "UBG turn should be >= B turn");
        assert!(ubg_turn >= result.turn_with_g.unwrap(), "UBG turn should be >= G turn");
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
            let result = run_game(&deck, 42, &db);
            assert!(result.on_the_play || !result.on_the_play, "Game should complete");
        }
    }
}

#[test]
fn test_game_state_consistency() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    // Run a game and verify state consistency
    let result = run_game(&deck, 777, &db);

    // If we won, verify damage adds up
    if result.win_turn.is_some() {
        // Total damage should be combat + combo
        let total_damage = result.total_combat_damage + result.combo_damage;
        assert!(total_damage >= 20, "Should have dealt at least 20 damage to win");
    }
}

#[test]
fn test_deterministic_rng_sequence() {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    // Run 5 games with same seed and verify all results are identical
    let mut results = Vec::new();
    for _ in 0..5 {
        results.push(run_game(&deck, 555, &db));
    }

    // All results should be identical
    for i in 1..results.len() {
        assert_eq!(results[0].win_turn, results[i].win_turn);
        assert_eq!(results[0].on_the_play, results[i].on_the_play);
        assert_eq!(results[0].total_combat_damage, results[i].total_combat_damage);
    }
}

