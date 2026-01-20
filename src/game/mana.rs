use crate::card::{Card, CreatureCard, ManaCost, ManaColor};
use crate::game::state::GameState;
use crate::game::zones::Permanent;

/// Mana pool tracking each color and colorless mana
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaPool {
    pub white: u32,
    pub blue: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub colorless: u32,
}

impl ManaPool {
    pub fn new() -> Self {
        ManaPool {
            white: 0,
            blue: 0,
            black: 0,
            red: 0,
            green: 0,
            colorless: 0,
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    /// Add mana of a specific color
    pub fn add_mana(&mut self, color: char, amount: u32) {
        match color {
            'W' => self.white += amount,
            'U' => self.blue += amount,
            'B' => self.black += amount,
            'R' => self.red += amount,
            'G' => self.green += amount,
            'C' => self.colorless += amount,
            _ => {}
        }
    }

    /// Get total mana available
    pub fn total(&self) -> u32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
    }

    /// Check if we can pay a mana cost
    pub fn can_pay(&self, cost: &ManaCost) -> bool {
        // Check colored requirements
        if cost.white > self.white {
            return false;
        }
        if cost.blue > self.blue {
            return false;
        }
        if cost.black > self.black {
            return false;
        }
        if cost.red > self.red {
            return false;
        }
        if cost.green > self.green {
            return false;
        }
        if cost.colorless > self.colorless {
            return false;
        }

        // Check if we have enough remaining for generic
        let remaining = self.white - cost.white
            + self.blue - cost.blue
            + self.black - cost.black
            + self.red - cost.red
            + self.green - cost.green
            + self.colorless - cost.colorless;

        remaining >= cost.generic
    }

    /// Pay a mana cost from the pool
    pub fn pay(&mut self, cost: &ManaCost) -> bool {
        if !self.can_pay(cost) {
            return false;
        }

        // Pay colored costs first
        self.white -= cost.white;
        self.blue -= cost.blue;
        self.black -= cost.black;
        self.red -= cost.red;
        self.green -= cost.green;
        self.colorless -= cost.colorless;

        // Pay generic with remaining mana (prefer colorless, then excess colors)
        let mut generic_remaining = cost.generic;
        let colors = ['C', 'W', 'U', 'B', 'R', 'G'];

        for color in &colors {
            if generic_remaining == 0 {
                break;
            }

            let available = match color {
                'W' => self.white,
                'U' => self.blue,
                'B' => self.black,
                'R' => self.red,
                'G' => self.green,
                'C' => self.colorless,
                _ => 0,
            };

            let to_pay = std::cmp::min(available, generic_remaining);
            match color {
                'W' => self.white -= to_pay,
                'U' => self.blue -= to_pay,
                'B' => self.black -= to_pay,
                'R' => self.red -= to_pay,
                'G' => self.green -= to_pay,
                'C' => self.colorless -= to_pay,
                _ => {}
            }
            generic_remaining -= to_pay;
        }

        true
    }

    /// Clear the mana pool
    pub fn clear(&mut self) {
        self.white = 0;
        self.blue = 0;
        self.black = 0;
        self.red = 0;
        self.green = 0;
        self.colorless = 0;
    }
}

impl Default for ManaPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the colors a land can tap for
/// Handles special lands like Cavern of Souls, Verge lands, Starting Town
pub fn can_tap_for_mana(
    permanent: &Permanent,
    state: &GameState,
    for_creature: Option<&CreatureCard>,
) -> Vec<ManaColor> {
    if permanent.tapped {
        return Vec::new();
    }

    let land = match &permanent.card {
        Card::Land(l) => l,
        _ => return Vec::new(),
    };

    // Handle Cavern of Souls - colored mana ONLY for creatures of chosen type
    if land.base.name == "Cavern of Souls" {
        // Cavern always produces {C}
        // Cavern produces any color ONLY for creatures of the chosen type
        if let (Some(creature), Some(chosen_type)) = (for_creature, &permanent.chosen_type) {
            if creature_matches_cavern_type(creature, chosen_type) {
                // Creature matches! Can produce any color
                return vec![
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                    ManaColor::Colorless,
                ];
            }
        }
        // No creature context or creature doesn't match - only colorless
        return vec![ManaColor::Colorless];
    }

    // Handle Wastewood Verge - {B} only if controlling Swamp/Forest
    if land.base.name == "Wastewood Verge" {
        let has_swamp_or_forest = state
            .battlefield
            .permanents()
            .iter()
            .any(|p| {
                if let Card::Land(l) = &p.card {
                    matches!(
                        l.base.name.as_str(),
                        "Swamp"
                            | "Forest"
                            | "Watery Grave"
                            | "Underground Mortuary"
                            | "Undercity Sewers"
                    )
                } else {
                    false
                }
            });
        if has_swamp_or_forest {
            return vec![ManaColor::Green, ManaColor::Black];
        }
        return vec![ManaColor::Green];
    }

    // Handle Gloomlake Verge - {B} only if controlling Island/Swamp
    if land.base.name == "Gloomlake Verge" {
        let has_island_or_swamp = state
            .battlefield
            .permanents()
            .iter()
            .any(|p| {
                if let Card::Land(l) = &p.card {
                    matches!(
                        l.base.name.as_str(),
                        "Island" | "Swamp" | "Watery Grave" | "Undercity Sewers"
                    )
                } else {
                    false
                }
            });
        if has_island_or_swamp {
            return vec![ManaColor::Blue, ManaColor::Black];
        }
        return vec![ManaColor::Blue];
    }

    // Handle Multiversal Passage - produces chosen color
    if land.base.name == "Multiversal Passage" {
        if let Some(chosen_type) = &permanent.chosen_basic_type {
            if let Ok(color) = parse_mana_color(chosen_type) {
                return vec![color];
            }
        }
    }

    // Handle Starting Town - produces C for free, or any color for 1 life
    if land.base.name == "Starting Town" {
        if state.life > 1 {
            // Can pay 1 life for any color
            return vec![
                ManaColor::Colorless,
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ];
        }
        // Only colorless if we can't afford the life
        return vec![ManaColor::Colorless];
    }

    // Return land colors for other lands
    land.colors.clone()
}

/// Check if a creature matches a Cavern of Souls chosen type
fn creature_matches_cavern_type(creature: &CreatureCard, chosen_type: &str) -> bool {
    creature.creature_types.iter().any(|t| t == chosen_type)
}

/// Parse a mana color string to ManaColor enum
fn parse_mana_color(color_str: &str) -> Result<ManaColor, String> {
    match color_str {
        "W" => Ok(ManaColor::White),
        "U" => Ok(ManaColor::Blue),
        "B" => Ok(ManaColor::Black),
        "R" => Ok(ManaColor::Red),
        "G" => Ok(ManaColor::Green),
        "C" => Ok(ManaColor::Colorless),
        _ => Err(format!("Unknown mana color: {}", color_str)),
    }
}

/// Check if we can afford a mana cost given the current game state
/// This checks if we have enough untapped lands to produce the required colors
pub fn can_afford_cost(
    cost: &ManaCost,
    state: &GameState,
    for_creature: Option<&CreatureCard>,
) -> bool {
    let max_mana = state
        .battlefield
        .permanents()
        .iter()
        .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
        .count() as u32;

    // Quick check: do we have enough total mana?
    let total_cost = cost.white + cost.blue + cost.black + cost.red + cost.green + cost.colorless + cost.generic;

    if max_mana < total_cost {
        return false;
    }

    // Check if we can produce each required color
    let mut color_counts = ManaPool::new();

    for permanent in state.battlefield.permanents() {
        if permanent.tapped {
            continue;
        }
        if !matches!(permanent.card, Card::Land(_)) {
            continue;
        }
        let colors = can_tap_for_mana(permanent, state, for_creature);
        for color in colors {
            match color {
                ManaColor::White => color_counts.white += 1,
                ManaColor::Blue => color_counts.blue += 1,
                ManaColor::Black => color_counts.black += 1,
                ManaColor::Red => color_counts.red += 1,
                ManaColor::Green => color_counts.green += 1,
                ManaColor::Colorless => color_counts.colorless += 1,
            }
        }
    }

    // Check colored requirements
    if cost.white > 0 && color_counts.white < cost.white {
        return false;
    }
    if cost.blue > 0 && color_counts.blue < cost.blue {
        return false;
    }
    if cost.black > 0 && color_counts.black < cost.black {
        return false;
    }
    if cost.red > 0 && color_counts.red < cost.red {
        return false;
    }
    if cost.green > 0 && color_counts.green < cost.green {
        return false;
    }

    true
}

/// Check if a spell can be cast with the current game state
pub fn can_cast_spell(card: &Card, state: &GameState) -> bool {
    match card {
        Card::Land(_) => false,
        Card::Creature(c) => {
            let for_creature = Some(c);

            // For creatures with impending, check if we can cast for impending cost
            if let Some(impending_cost) = &c.impending_cost {
                if can_afford_cost(impending_cost, state, for_creature) {
                    return true;
                }
            }

            // Check regular mana cost
            can_afford_cost(&c.base.mana_cost, state, for_creature)
        }
        Card::Instant(c) => can_afford_cost(&c.base.mana_cost, state, None),
        Card::Sorcery(c) => can_afford_cost(&c.base.mana_cost, state, None),
        Card::Enchantment(c) => can_afford_cost(&c.base.mana_cost, state, None),
        Card::Saga(c) => can_afford_cost(&c.base.mana_cost, state, None),
    }
}

/// Tap lands to pay a mana cost. Returns true if successful.
/// This is the key function that taps lands DURING casting, not before.
/// Strategy: Use lands that only produce the required colors first (to preserve flexibility)
pub fn tap_lands_for_cost(
    cost: &ManaCost,
    state: &mut GameState,
    for_creature: Option<&CreatureCard>,
) -> bool {
    // First check if we can afford the cost
    if !can_afford_cost(cost, state, for_creature) {
        return false;
    }

    // Collect all land info FIRST (before any mutations)
    // Each entry is (index, colors_this_land_produces)
    let land_info: Vec<(usize, Vec<ManaColor>)> = state.battlefield.permanents()
        .iter()
        .enumerate()
        .filter_map(|(idx, p)| {
            if p.tapped || !matches!(p.card, Card::Land(_)) {
                return None;
            }
            let colors = can_tap_for_mana(p, state, for_creature);
            if colors.is_empty() {
                return None;
            }
            Some((idx, colors))
        })
        .collect();

    // Track which lands we'll tap (by index)
    let mut lands_to_tap: Vec<(usize, char)> = Vec::new();
    let mut used_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // Pay colored costs first
    let colors_to_pay = [
        (ManaColor::White, cost.white),
        (ManaColor::Blue, cost.blue),
        (ManaColor::Black, cost.black),
        (ManaColor::Red, cost.red),
        (ManaColor::Green, cost.green),
        (ManaColor::Colorless, cost.colorless),
    ];

    for (color, amount) in &colors_to_pay {
        let mut remaining = *amount;

        // Prefer lands that ONLY produce this color (preserve flexibility)
        for (idx, colors) in &land_info {
            if remaining == 0 {
                break;
            }
            if used_indices.contains(idx) {
                continue;
            }
            if colors.len() == 1 && colors[0] == *color {
                lands_to_tap.push((*idx, color.to_char()));
                used_indices.insert(*idx);
                remaining -= 1;
            }
        }

        // Then use multi-color lands
        if remaining > 0 {
            for (idx, colors) in &land_info {
                if remaining == 0 {
                    break;
                }
                if used_indices.contains(idx) {
                    continue;
                }
                if colors.contains(color) {
                    lands_to_tap.push((*idx, color.to_char()));
                    used_indices.insert(*idx);
                    remaining -= 1;
                }
            }
        }

        // If we still have remaining, something went wrong with can_afford_cost
        if remaining > 0 {
            return false;
        }
    }

    // Pay generic with remaining untapped lands
    let mut generic_remaining = cost.generic;
    for (idx, colors) in &land_info {
        if generic_remaining == 0 {
            break;
        }
        if used_indices.contains(idx) {
            continue;
        }
        if !colors.is_empty() {
            lands_to_tap.push((*idx, colors[0].to_char()));
            used_indices.insert(*idx);
            generic_remaining -= 1;
        }
    }

    // Now actually tap the lands and add mana to pool
    for (idx, color_char) in lands_to_tap {
        if let Some(perm) = state.battlefield.permanents_mut().get_mut(idx) {
            perm.tapped = true;
            state.mana_pool.add_mana(color_char, 1);
        }
    }

    // Now pay the actual cost from the pool
    state.mana_pool.pay(cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_mana() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 2);
        pool.add_mana('U', 1);
        assert_eq!(pool.white, 2);
        assert_eq!(pool.blue, 1);
        assert_eq!(pool.total(), 3);
    }

    #[test]
    fn test_can_pay_exact() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 2);
        pool.add_mana('U', 1);

        let cost = ManaCost {
            white: 2,
            blue: 1,
            ..Default::default()
        };

        assert!(pool.can_pay(&cost));
    }

    #[test]
    fn test_can_pay_with_generic() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 3);

        let cost = ManaCost {
            white: 1,
            generic: 2,
            ..Default::default()
        };

        assert!(pool.can_pay(&cost));
    }

    #[test]
    fn test_cannot_pay_insufficient() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 1);

        let cost = ManaCost {
            white: 2,
            ..Default::default()
        };

        assert!(!pool.can_pay(&cost));
    }

    #[test]
    fn test_pay_mana() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 3);
        pool.add_mana('U', 1);

        let cost = ManaCost {
            white: 1,
            generic: 2,
            ..Default::default()
        };

        assert!(pool.pay(&cost));
        assert_eq!(pool.white, 0);
        assert_eq!(pool.blue, 1);
    }
}

