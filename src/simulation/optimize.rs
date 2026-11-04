use std::collections::HashMap;
use crate::card::{Card, CardDatabase};
use crate::rng::GameRng;
use crate::simulation::deck::parse_deck_file;
use crate::simulation::engine::run_game;
use rayon::prelude::*;

/// Land configuration: map of land name to count
pub type LandConfig = HashMap<String, usize>;

/// Fixed cards configuration: map of card name to count (extracted from deck file)
pub type FixedCards = Vec<(String, usize)>;

/// Land type definition with constraints
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Default land pool, used when no --pool file is given.
/// This is the pool for the old U/B/G deck.
pub fn default_land_types() -> Vec<LandType> {
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

/// Parse a land pool file.
/// Each line: `<count-min> <count-max> <Land name>`.
/// Blank lines and lines that start with `#` or `//` are skipped.
pub fn parse_pool_file(path: &str) -> Result<Vec<LandType>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read pool file '{}': {}", path, e))?;
    parse_pool_text(&content)
}

/// Parse land pool text (see `parse_pool_file` for the format).
pub fn parse_pool_text(content: &str) -> Result<Vec<LandType>, String> {
    let mut pool = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        if parts.len() != 3 {
            return Err(format!(
                "Pool line {}: expected '<count-min> <count-max> <Land name>', got '{}'",
                line_num + 1, trimmed
            ));
        }

        let min: usize = parts[0].parse()
            .map_err(|_| format!("Pool line {}: '{}' is not a valid count", line_num + 1, parts[0]))?;
        let max: usize = parts[1].parse()
            .map_err(|_| format!("Pool line {}: '{}' is not a valid count", line_num + 1, parts[1]))?;
        let name = parts[2].trim().to_string();

        if min > max {
            return Err(format!("Pool line {}: min {} > max {}", line_num + 1, min, max));
        }
        if pool.iter().any(|l: &LandType| l.name == name) {
            return Err(format!("Pool line {}: duplicate land '{}'", line_num + 1, name));
        }

        pool.push(LandType { name, min, max });
    }

    let min_sum: usize = pool.iter().map(|l| l.min).sum();
    let max_sum: usize = pool.iter().map(|l| l.max).sum();
    if min_sum > TOTAL_LANDS {
        return Err(format!("Pool minimums add up to {}, more than {} lands", min_sum, TOTAL_LANDS));
    }
    if max_sum < TOTAL_LANDS {
        return Err(format!("Pool maximums add up to {}, fewer than {} lands", max_sum, TOTAL_LANDS));
    }

    Ok(pool)
}

/// Generate a random land configuration from a pool.
/// Minimums are placed first; the remaining slots are filled by shuffling
/// the spare capacity (max - min copies of each land) and taking the first N.
pub fn generate_random_land_config(pool: &[LandType], rng: &mut GameRng) -> LandConfig {
    let mut config = LandConfig::new();
    let mut remaining = TOTAL_LANDS;

    // First: enforce minimum constraints
    for land in pool {
        if land.min > 0 {
            config.insert(land.name.clone(), land.min);
            remaining -= land.min;
        }
    }

    // Pool of spare capacity for each land
    let mut spare: Vec<&str> = Vec::new();
    for land in pool {
        for _ in 0..(land.max - land.min) {
            spare.push(&land.name);
        }
    }

    rng.shuffle(&mut spare);

    for name in spare.iter().take(remaining) {
        *config.entry((*name).to_string()).or_insert(0) += 1;
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

/// Result of evaluating one land configuration over a set of seeded games
#[derive(Clone, Debug)]
pub struct EvalResult {
    pub avg_win_turn: f64,
    pub win_rate: f64,
    /// Share of games won on turn N (percent of all games)
    pub turn_win_pct: HashMap<u32, f64>,
    /// Wrong mana on turn 4 (percent of all games)
    pub wrong_mana_turn4_pct: f64,
    pub turn_distribution: HashMap<u32, usize>,
    pub games: usize,
}

impl EvalResult {
    pub fn turn_pct(&self, turn: u32) -> f64 {
        self.turn_win_pct.get(&turn).copied().unwrap_or(0.0)
    }

    /// Turn 4 + turn 5 win percent, the tie-break metric
    pub fn t45_pct(&self) -> f64 {
        self.turn_pct(4) + self.turn_pct(5)
    }

    /// Score order: lower average win turn first;
    /// on a tie, higher turn-4 + turn-5 win percent first.
    pub fn score_cmp(&self, other: &EvalResult) -> std::cmp::Ordering {
        self.avg_win_turn
            .partial_cmp(&other.avg_win_turn)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                other.t45_pct()
                    .partial_cmp(&self.t45_pct())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

/// Evaluate a deck over `games` games. Game i uses seed `base_seed + i`,
/// so every configuration plays the same shuffles and results repeat.
pub fn evaluate_deck(deck: &[Card], db: &CardDatabase, games: usize, base_seed: u64) -> EvalResult {
    let results: Vec<_> = (0..games)
        .into_par_iter()
        .map(|i| run_game(deck, base_seed + i as u64, db, false))
        .collect();

    let wins: Vec<u32> = results.iter().filter_map(|r| r.win_turn).collect();
    let win_rate = wins.len() as f64 / games as f64;
    let avg_win_turn = if wins.is_empty() {
        f64::INFINITY
    } else {
        wins.iter().map(|t| *t as f64).sum::<f64>() / wins.len() as f64
    };

    let mut turn_distribution: HashMap<u32, usize> = HashMap::new();
    for t in &wins {
        *turn_distribution.entry(*t).or_insert(0) += 1;
    }
    let turn_win_pct: HashMap<u32, f64> = turn_distribution
        .iter()
        .map(|(t, c)| (*t, *c as f64 / games as f64 * 100.0))
        .collect();

    let wrong_mana_games = results.iter()
        .filter(|r| r.turn4_mana.map_or(false, |m| m.wrong_mana))
        .count();
    let wrong_mana_turn4_pct = wrong_mana_games as f64 / games as f64 * 100.0;

    EvalResult {
        avg_win_turn,
        win_rate,
        turn_win_pct,
        wrong_mana_turn4_pct,
        turn_distribution,
        games,
    }
}

/// Evaluate a land configuration (fixed cards + lands) over seeded games.
pub fn evaluate_config(
    config: &LandConfig,
    fixed_cards: &FixedCards,
    db: &CardDatabase,
    games: usize,
    base_seed: u64,
) -> Result<EvalResult, String> {
    let deck = build_deck_from_config_with_fixed(config, fixed_cards, db)?;
    Ok(evaluate_deck(&deck, db, games, base_seed))
}

/// All single swaps from `config`: -1 of land X, +1 of land Y,
/// within the pool's min/max limits.
pub fn neighbor_swaps(config: &LandConfig, pool: &[LandType]) -> Vec<(String, String, LandConfig)> {
    let mut swaps = Vec::new();
    for donor in pool {
        let donor_count = config.get(&donor.name).copied().unwrap_or(0);
        if donor_count <= donor.min {
            continue;
        }
        for receiver in pool {
            if receiver.name == donor.name {
                continue;
            }
            let receiver_count = config.get(&receiver.name).copied().unwrap_or(0);
            if receiver_count >= receiver.max {
                continue;
            }
            let mut next = config.clone();
            *next.get_mut(&donor.name).unwrap() -= 1;
            *next.entry(receiver.name.clone()).or_insert(0) += 1;
            next.retain(|_, c| *c > 0);
            swaps.push((donor.name.clone(), receiver.name.clone(), next));
        }
    }
    swaps
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

/// Read Arena set/collector suffixes ("(SET) 123") per card name from a deck file.
pub fn parse_set_codes(deck_file: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string(deck_file) else {
        return map;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let Some((_, rest)) = trimmed.split_once(' ') else { continue };
        let rest = rest.trim();
        if let Some(paren_pos) = rest.find(" (") {
            let name = rest[..paren_pos].trim().to_string();
            let suffix = rest[paren_pos..].trim().to_string();
            map.insert(name, suffix);
        }
    }
    map
}

/// Fallback Arena set codes for lands that may not appear in the base deck file.
fn fallback_set_code(name: &str) -> Option<&'static str> {
    match name {
        "Island" => Some("(FDN) 272"),
        "Swamp" => Some("(FDN) 276"),
        "Forest" => Some("(FDN) 280"),
        "Shipwreck Marsh" => Some("(FRA) 189"),
        "Watery Grave" => Some("(EOE) 261"),
        "Undercity Sewers" => Some("(MKM) 270"),
        "Gloomlake Verge" => Some("(DSK) 260"),
        "Multiversal Passage" => Some("(SPM) 180"),
        "Starting Town" => Some("(FIN) 289"),
        "Cavern of Souls" => Some("(LCI) 269"),
        "Pit of Offerings" => Some("(LCI) 278"),
        _ => None,
    }
}

fn arena_line(name: &str, count: usize, set_codes: &HashMap<String, String>) -> String {
    let suffix = set_codes
        .get(name)
        .map(|s| s.as_str())
        .or_else(|| fallback_set_code(name));
    match suffix {
        Some(s) => format!("{} {} {}", count, name, s),
        None => format!("{} {}", count, name),
    }
}

/// Parameters for saving a deck configuration
pub struct DeckSaveParams<'a> {
    pub eval: &'a EvalResult,
    pub base_seed: u64,
    pub strategy: String,
    pub fixed_cards: &'a FixedCards,
    /// Arena "(SET) 123" suffixes per card name, usually from the base deck file
    pub set_codes: &'a HashMap<String, String>,
    /// Output file name; when None, deck_<hash>.txt is used
    pub out_file: Option<String>,
}

/// Save a deck configuration to a file in Arena format with a metadata header.
pub fn save_deck_to_file(config: &LandConfig, params: &DeckSaveParams) -> std::io::Result<String> {
    use std::fs::File;
    use std::io::Write;
    use chrono::Local;

    let hash = calculate_deck_hash_with_fixed(config, params.fixed_cards);
    let filename = params
        .out_file
        .clone()
        .unwrap_or_else(|| format!("deck_{}.txt", hash));

    let mut file = File::create(&filename)?;

    // Write header with metadata
    writeln!(file, "# MTG Reanimator Deck")?;
    writeln!(file, "# Generated: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "# Hash: {}", hash)?;
    writeln!(file, "#")?;

    // Optimization parameters
    writeln!(file, "# Optimization Results")?;
    writeln!(file, "# Strategy: {}", params.strategy)?;
    writeln!(file, "# Simulations: {}", params.eval.games)?;
    writeln!(file, "# Seed base: {}", params.base_seed)?;
    writeln!(file, "# Win rate: {:.1}%", params.eval.win_rate * 100.0)?;
    writeln!(file, "# Average win turn: {:.3}", params.eval.avg_win_turn)?;
    writeln!(file, "# Wrong mana on turn 4: {:.2}%", params.eval.wrong_mana_turn4_pct)?;
    writeln!(file, "#")?;

    // Turn distribution
    writeln!(file, "# Turn Distribution")?;
    let mut turns: Vec<_> = params.eval.turn_distribution.iter().collect();
    turns.sort_by_key(|(turn, _)| *turn);
    let total_wins: usize = params.eval.turn_distribution.values().sum();
    for (turn, count) in turns {
        let pct = if total_wins > 0 { *count as f64 / total_wins as f64 * 100.0 } else { 0.0 };
        writeln!(file, "# Turn {}: {} ({:.1}%)", turn, count, pct)?;
    }

    // Arena format: "Deck" header, then card lines with set codes
    writeln!(file, "Deck")?;

    // Calculate total fixed cards
    let fixed_card_count: usize = params.fixed_cards.iter().map(|(_, count)| count).sum();

    // Write fixed cards first
    writeln!(file, "# Fixed cards ({})", fixed_card_count)?;
    let mut sorted_fixed: Vec<_> = params.fixed_cards.iter().collect();
    sorted_fixed.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, count) in sorted_fixed {
        writeln!(file, "{}", arena_line(name, *count, params.set_codes))?;
    }

    writeln!(file)?;

    // Write lands sorted by count then name
    writeln!(file, "# Lands ({})", TOTAL_LANDS)?;
    let mut lands: Vec<_> = config.iter().filter(|(_, count)| **count > 0).collect();
    lands.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (name, count) in lands {
        writeln!(file, "{}", arena_line(name, *count, params.set_codes))?;
    }

    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POOL_TEXT: &str = "\
# test pool
0 4 Watery Grave
0 8 Swamp
0 4 Island
2 2 Cavern of Souls
1 1 Pit of Offerings
0 4 Gloomlake Verge
0 4 Undercity Sewers
0 4 Shipwreck Marsh
0 4 Multiversal Passage
0 4 Starting Town
";

    #[test]
    fn test_parse_pool_text() {
        let pool = parse_pool_text(POOL_TEXT).expect("pool should parse");
        assert_eq!(pool.len(), 10);
        assert_eq!(pool[0], LandType { name: "Watery Grave".to_string(), min: 0, max: 4 });
        assert_eq!(pool[1], LandType { name: "Swamp".to_string(), min: 0, max: 8 });
        let cavern = pool.iter().find(|l| l.name == "Cavern of Souls").unwrap();
        assert_eq!((cavern.min, cavern.max), (2, 2));
        let pit = pool.iter().find(|l| l.name == "Pit of Offerings").unwrap();
        assert_eq!((pit.min, pit.max), (1, 1));
    }

    #[test]
    fn test_parse_pool_text_errors() {
        assert!(parse_pool_text("4 Island").is_err(), "missing field");
        assert!(parse_pool_text("3 1 Island").is_err(), "min > max");
        assert!(parse_pool_text("0 4 Island\n0 4 Island").is_err(), "duplicate");
        assert!(parse_pool_text("0 4 Island").is_err(), "maximums below 24");
    }

    #[test]
    fn test_generated_configs_respect_limits() {
        let pool = parse_pool_text(POOL_TEXT).expect("pool should parse");
        let mut rng = GameRng::new(Some(42));
        for _ in 0..100 {
            let config = generate_random_land_config(&pool, &mut rng);
            let total: usize = config.values().sum();
            assert_eq!(total, TOTAL_LANDS, "total lands must be {}", TOTAL_LANDS);
            for land in &pool {
                let count = config.get(&land.name).copied().unwrap_or(0);
                assert!(count >= land.min, "{} below min: {} < {}", land.name, count, land.min);
                assert!(count <= land.max, "{} above max: {} > {}", land.name, count, land.max);
            }
        }
    }

    #[test]
    fn test_default_pool_still_works() {
        let pool = default_land_types();
        let mut rng = GameRng::new(Some(7));
        let config = generate_random_land_config(&pool, &mut rng);
        let total: usize = config.values().sum();
        assert_eq!(total, TOTAL_LANDS);
        assert_eq!(config.get("Cavern of Souls").copied().unwrap_or(0), 4);
    }

    #[test]
    fn test_same_seed_same_result() {
        let db = CardDatabase::from_file("cards.json").expect("cards.json should load");
        let fixed = extract_fixed_cards_from_deck("deck_v_roiling_2swamp.txt", &db)
            .expect("deck should parse");
        let pool = parse_pool_text(POOL_TEXT).expect("pool should parse");
        let mut rng = GameRng::new(Some(5));
        let config = generate_random_land_config(&pool, &mut rng);

        let a = evaluate_config(&config, &fixed, &db, 200, 1).expect("eval a");
        let b = evaluate_config(&config, &fixed, &db, 200, 1).expect("eval b");
        assert_eq!(a.avg_win_turn, b.avg_win_turn);
        assert_eq!(a.win_rate, b.win_rate);
        assert_eq!(a.turn_distribution, b.turn_distribution);

        let c = evaluate_config(&config, &fixed, &db, 200, 999).expect("eval c");
        assert!(
            a.avg_win_turn != c.avg_win_turn || a.turn_distribution != c.turn_distribution,
            "different seed bases should give different games"
        );
    }

    #[test]
    fn test_neighbor_swaps_respect_limits() {
        let pool = parse_pool_text(POOL_TEXT).expect("pool should parse");
        let mut rng = GameRng::new(Some(11));
        let config = generate_random_land_config(&pool, &mut rng);
        let swaps = neighbor_swaps(&config, &pool);
        assert!(!swaps.is_empty());
        for (donor, receiver, next) in &swaps {
            assert_ne!(donor, receiver);
            let total: usize = next.values().sum();
            assert_eq!(total, TOTAL_LANDS);
            for land in &pool {
                let count = next.get(&land.name).copied().unwrap_or(0);
                assert!(count >= land.min && count <= land.max,
                    "{} out of range after swap {} -> {}", land.name, donor, receiver);
            }
        }
    }
}
