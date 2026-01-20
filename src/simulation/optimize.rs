use std::collections::HashMap;
use crate::card::{Card, CardDatabase};
use crate::rng::GameRng;

/// Land configuration: map of land name to count
pub type LandConfig = HashMap<String, usize>;

/// Land type definition with constraints
#[derive(Clone, Debug)]
pub struct LandType {
    pub name: String,
    pub max: usize,
}

/// Fixed non-land cards that stay the same across all configurations
pub const FIXED_CARDS: &[(&str, usize)] = &[
    ("Terror of the Peaks", 4),
    ("Bringer of the Last Gift", 4),
    ("Superior Spider-Man", 4),
    ("Overlord of the Balemurk", 4),
    ("Kiora, the Rising Tide", 4),
    ("Town Greeter", 3),
    ("Cache Grab", 4),
    ("Dredger's Insight", 4),
    ("Awaken the Honored Dead", 4),
    ("Analyze the Pollen", 1),
];

pub const TOTAL_LANDS: usize = 24; // 60 - 36

/// Get all available land types with their constraints
pub fn get_land_types() -> Vec<LandType> {
    vec![
        LandType { name: "Forest".to_string(), max: 4 },
        LandType { name: "Island".to_string(), max: 4 },
        LandType { name: "Swamp".to_string(), max: 4 },
        LandType { name: "Watery Grave".to_string(), max: 4 },
        LandType { name: "Undercity Sewers".to_string(), max: 4 },
        LandType { name: "Underground Mortuary".to_string(), max: 4 },
        LandType { name: "Cavern of Souls".to_string(), max: 4 },
        LandType { name: "Restless Cottage".to_string(), max: 1 },
        LandType { name: "Wastewood Verge".to_string(), max: 4 },
        LandType { name: "Gloomlake Verge".to_string(), max: 4 },
        LandType { name: "Multiversal Passage".to_string(), max: 4 },
        LandType { name: "Blooming Marsh".to_string(), max: 4 },
        LandType { name: "Starting Town".to_string(), max: 4 },
    ]
}

/// Generate a random land configuration using weighted strategy
pub fn generate_random_land_config_weighted(rng: &mut GameRng) -> LandConfig {
    let mut config = LandConfig::new();
    let mut remaining = TOTAL_LANDS;
    let mut land_types = get_land_types();

    // Shuffle land types randomly
    rng.shuffle(&mut land_types);

    // First pass: assign random counts respecting max limits
    for land in &land_types {
        let max_allowed = std::cmp::min(land.max, remaining);
        let count = rng.random_range(max_allowed + 1);
        config.insert(land.name.clone(), count);
        remaining -= count;
    }

    // Second pass: distribute remaining slots
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

    // Create pool with max copies of each land
    let mut pool: Vec<String> = Vec::new();
    for land in &land_types {
        for _ in 0..land.max {
            pool.push(land.name.clone());
        }
    }

    // Shuffle the pool
    rng.shuffle(&mut pool);

    // Take first TOTAL_LANDS
    for i in 0..TOTAL_LANDS {
        let land_name = pool[i].clone();
        *config.entry(land_name).or_insert(0) += 1;
    }

    config
}

/// Build a complete deck from a land configuration
pub fn build_deck_from_config(config: &LandConfig, db: &CardDatabase) -> Result<Vec<Card>, String> {
    let mut cards = Vec::new();

    // Add fixed cards
    for (card_name, count) in FIXED_CARDS {
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

