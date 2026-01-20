use std::collections::HashMap;
use crate::card::{Card, CardDatabase};
use crate::rng::GameRng;
use crate::simulation::deck::parse_deck_file;

/// Land configuration: map of land name to count
pub type LandConfig = HashMap<String, usize>;

/// Fixed cards configuration: map of card name to count (extracted from deck file)
pub type FixedCards = Vec<(String, usize)>;

/// Land type definition with constraints
#[derive(Clone, Debug)]
pub struct LandType {
    pub name: String,
    pub min: usize,
    pub max: usize,
}

pub const TOTAL_LANDS: usize = 24; // 60 - 36

/// Extract non-land cards from a deck file
pub fn extract_fixed_cards_from_deck(deck_file: &str, db: &CardDatabase) -> Result<FixedCards, String> {
    let deck = parse_deck_file(deck_file, db).map_err(|e| format!("{:?}", e))?;

    // Count each non-land card
    let mut card_counts: HashMap<String, usize> = HashMap::new();
    for card in deck {
        if !matches!(card, Card::Land(_)) {
            *card_counts.entry(card.name().to_string()).or_insert(0) += 1;
        }
    }

    // Convert to sorted vector for consistent ordering
    let mut fixed_cards: FixedCards = card_counts.into_iter().collect();
    fixed_cards.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(fixed_cards)
}

/// Get all available land types with their constraints
pub fn get_land_types() -> Vec<LandType> {
    vec![
        LandType { name: "Forest".to_string(), min: 0, max: 4 },
        LandType { name: "Island".to_string(), min: 0, max: 4 },
        LandType { name: "Swamp".to_string(), min: 0, max: 4 },
        LandType { name: "Watery Grave".to_string(), min: 0, max: 4 },
        LandType { name: "Undercity Sewers".to_string(), min: 0, max: 4 },
        LandType { name: "Underground Mortuary".to_string(), min: 0, max: 4 },
        // 4 Cavern of Souls for anti-counterspell protection
        LandType { name: "Cavern of Souls".to_string(), min: 4, max: 4 },
        LandType { name: "Restless Cottage".to_string(), min: 0, max: 1 },
        LandType { name: "Wastewood Verge".to_string(), min: 0, max: 4 },
        LandType { name: "Gloomlake Verge".to_string(), min: 0, max: 4 },
        LandType { name: "Multiversal Passage".to_string(), min: 0, max: 4 },
        LandType { name: "Blooming Marsh".to_string(), min: 0, max: 4 },
        LandType { name: "Starting Town".to_string(), min: 0, max: 4 },
    ]
}

/// Generate a random land configuration using weighted strategy
pub fn generate_random_land_config_weighted(rng: &mut GameRng) -> LandConfig {
    let mut config = LandConfig::new();
    let mut remaining = TOTAL_LANDS;
    let mut land_types = get_land_types();

    // First pass: enforce minimum constraints
    for land in &land_types {
        if land.min > 0 {
            config.insert(land.name.clone(), land.min);
            remaining -= land.min;
        }
    }

    // Shuffle land types randomly for variety
    rng.shuffle(&mut land_types);

    // Second pass: assign random counts respecting max limits
    for land in &land_types {
        let current = config.get(&land.name).copied().unwrap_or(0);
        let max_additional = std::cmp::min(land.max - current, remaining);
        let additional = rng.random_range(max_additional + 1);
        *config.entry(land.name.clone()).or_insert(0) += additional;
        remaining -= additional;
    }

    // Third pass: distribute remaining slots
    let mut attempts = 0;
    while remaining > 0 && attempts < 1000 {
        let idx = rng.random_range(land_types.len());
        let land = &land_types[idx];
        if config.get(&land.name).copied().unwrap_or(0) < land.max {
            *config.entry(land.name.clone()).or_insert(0) += 1;
            remaining -= 1;
        }
        attempts += 1;
    }

    config
}

/// Generate a random land configuration using shuffle strategy
pub fn generate_random_land_config_shuffle(rng: &mut GameRng) -> LandConfig {
    let mut config = LandConfig::new();
    let land_types = get_land_types();
    let mut remaining = TOTAL_LANDS;

    // First: enforce minimum constraints
    for land in &land_types {
        if land.min > 0 {
            config.insert(land.name.clone(), land.min);
            remaining -= land.min;
        }
    }

    // Create pool with remaining capacity for each land (max - min already used)
    let mut pool: Vec<String> = Vec::new();
    for land in &land_types {
        let already_used = config.get(&land.name).copied().unwrap_or(0);
        for _ in 0..(land.max - already_used) {
            pool.push(land.name.clone());
        }
    }

    // Shuffle the pool
    rng.shuffle(&mut pool);

    // Take from shuffled pool to fill remaining slots
    for i in 0..remaining.min(pool.len()) {
        let land_name = pool[i].clone();
        *config.entry(land_name).or_insert(0) += 1;
    }

    config
}

/// Build a complete deck from a land configuration and fixed cards
pub fn build_deck_from_config_with_fixed(config: &LandConfig, fixed_cards: &FixedCards, db: &CardDatabase) -> Result<Vec<Card>, String> {
    let mut cards = Vec::new();

    // Add fixed cards
    for (card_name, count) in fixed_cards {
        for _ in 0..*count {
            match db.get_card(card_name) {
                Ok(card) => cards.push(card.clone()),
                Err(_) => return Err(format!("Card not found: {}", card_name)),
            }
        }
    }

    // Add lands from config
    for (land_name, count) in config {
        for _ in 0..*count {
            match db.get_card(land_name) {
                Ok(card) => cards.push(card.clone()),
                Err(_) => return Err(format!("Land not found: {}", land_name)),
            }
        }
    }

    Ok(cards)
}



/// Format a land configuration as a readable string
pub fn config_to_string(config: &LandConfig) -> String {
    let mut items: Vec<_> = config
        .iter()
        .filter(|(_, count)| **count > 0)
        .collect();

    items.sort_by(|a, b| {
        b.1.cmp(a.1).then_with(|| a.0.cmp(b.0))
    });

    items
        .iter()
        .map(|(name, count)| format!("{} {}", count, name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Calculate a short hash for a deck configuration with custom fixed cards
pub fn calculate_deck_hash_with_fixed(config: &LandConfig, fixed_cards: &FixedCards) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Create a sorted list of all cards (fixed + lands)
    let mut all_cards: Vec<(String, usize)> = fixed_cards.clone();

    for (name, count) in config {
        if *count > 0 {
            all_cards.push((name.clone(), *count));
        }
    }

    // Sort alphabetically for consistent hashing
    all_cards.sort_by(|a, b| a.0.cmp(&b.0));

    // Hash the sorted card list
    let mut hasher = DefaultHasher::new();
    for (name, count) in &all_cards {
        name.hash(&mut hasher);
        count.hash(&mut hasher);
    }

    // Return first 8 hex chars of hash
    format!("{:016x}", hasher.finish())[..8].to_string()
}



/// Parameters for saving a deck configuration
pub struct DeckSaveParams<'a> {
    pub win_rate: f64,
    pub avg_win_turn: f64,
    pub num_simulations: usize,
    pub strategy: String,
    pub turn_distribution: std::collections::HashMap<u32, usize>,
    pub fixed_cards: &'a FixedCards,
}

/// Save a deck configuration to a file with optimization results
pub fn save_deck_to_file(config: &LandConfig, params: &DeckSaveParams) -> std::io::Result<String> {
    use std::fs::File;
    use std::io::Write;
    use chrono::Local;

    let hash = calculate_deck_hash_with_fixed(config, params.fixed_cards);
    let filename = format!("deck_{}.txt", hash);

    let mut file = File::create(&filename)?;

    // Write header with metadata
    writeln!(file, "# MTG Reanimator Deck")?;
    writeln!(file, "# Generated: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "# Hash: {}", hash)?;
    writeln!(file, "#")?;

    // Optimization parameters
    writeln!(file, "# Optimization Results")?;
    writeln!(file, "# Strategy: {}", params.strategy)?;
    writeln!(file, "# Simulations: {}", params.num_simulations)?;
    writeln!(file, "# Win rate: {:.1}%", params.win_rate * 100.0)?;
    writeln!(file, "# Average win turn: {:.3}", params.avg_win_turn)?;
    writeln!(file, "#")?;

    // Turn distribution
    writeln!(file, "# Turn Distribution")?;
    let mut turns: Vec<_> = params.turn_distribution.iter().collect();
    turns.sort_by_key(|(turn, _)| *turn);
    let total_wins: usize = params.turn_distribution.values().sum();
    for (turn, count) in turns {
        let pct = if total_wins > 0 { *count as f64 / total_wins as f64 * 100.0 } else { 0.0 };
        writeln!(file, "# Turn {}: {} ({:.1}%)", turn, count, pct)?;
    }
    writeln!(file)?;

    // Calculate total fixed cards
    let fixed_card_count: usize = params.fixed_cards.iter().map(|(_, count)| count).sum();

    // Write fixed cards first
    writeln!(file, "# Fixed cards ({})", fixed_card_count)?;
    let mut sorted_fixed: Vec<_> = params.fixed_cards.iter().collect();
    sorted_fixed.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, count) in sorted_fixed {
        writeln!(file, "{} {}", count, name)?;
    }

    writeln!(file)?;

    // Write lands sorted by count then name
    writeln!(file, "# Lands (24)")?;
    let mut lands: Vec<_> = config.iter().filter(|(_, count)| **count > 0).collect();
    lands.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (name, count) in lands {
        writeln!(file, "{} {}", count, name)?;
    }

    Ok(filename)
}

