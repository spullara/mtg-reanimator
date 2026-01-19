mod card;
mod cli;
mod game;
mod rng;
mod simulation;

use card::CardDatabase;

fn main() {
    // Load the card database
    match CardDatabase::from_file("cards.json") {
        Ok(db) => {
            println!("✓ Loaded {} cards from cards.json", db.card_count());

            // Validate the database
            if let Err(e) = db.validate() {
                eprintln!("✗ Database validation failed: {}", e);
                std::process::exit(1);
            }

            println!("✓ Database validation passed");

            // Print all card names
            let names = db.card_names();
            println!("\nCards in database:");
            for name in names {
                println!("  - {}", name);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to load cards: {}", e);
            std::process::exit(1);
        }
    }
}
