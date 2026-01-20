use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mtg_reanimator::card::CardDatabase;
use mtg_reanimator::simulation::deck::parse_deck_file;
use mtg_reanimator::simulation::engine::run_game;

fn benchmark_single_game(c: &mut Criterion) {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");

    c.bench_function("single_game_seed_12345", |b| {
        b.iter(|| {
            run_game(black_box(&deck), black_box(12345), black_box(&db), black_box(false))
        })
    });
}

fn benchmark_multiple_games(c: &mut Criterion) {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    let deck = parse_deck_file("deck.txt", &db).expect("Failed to parse deck");
    
    c.bench_function("100_games", |b| {
        b.iter(|| {
            for seed in 0..100 {
                run_game(black_box(&deck), black_box(seed), black_box(&db));
            }
        })
    });
}

fn benchmark_deck_parsing(c: &mut Criterion) {
    let db = CardDatabase::from_file("cards.json").expect("Failed to load cards");
    
    c.bench_function("parse_deck_file", |b| {
        b.iter(|| {
            parse_deck_file(black_box("deck.txt"), black_box(&db))
        })
    });
}

criterion_group!(benches, benchmark_single_game, benchmark_multiple_games, benchmark_deck_parsing);
criterion_main!(benches);

