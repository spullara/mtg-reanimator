use crate::card::{Card, ColorFlags, CreatureCard, ManaCost, ManaColor};
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

/// Get the colors a land can tap for as bitflags (no allocations)
/// Handles special lands like Cavern of Souls, Verge lands, Starting Town
#[inline]
pub fn can_tap_for_mana(
    permanent: &Permanent,
    state: &GameState,
    for_creature: Option<&CreatureCard>,
) -> ColorFlags {
    if permanent.tapped {
        return ColorFlags::new();
    }

    let land = match &permanent.card {
        Card::Land(l) => l,
        _ => return ColorFlags::new(),
    };

    // Handle Cavern of Souls - colored mana ONLY for creatures of chosen type
    if land.base.name == "Cavern of Souls" {
        // Cavern always produces {C}
        // Cavern produces any color ONLY for creatures of the chosen type
        if let (Some(creature), Some(chosen_type)) = (for_creature, &permanent.chosen_type) {
            if creature_matches_cavern_type(creature, chosen_type) {
                // Creature matches! Can produce any color
                return ColorFlags(
                    ColorFlags::WHITE | ColorFlags::BLUE | ColorFlags::BLACK |
                    ColorFlags::RED | ColorFlags::GREEN | ColorFlags::COLORLESS
                );
            }
        }
        // No creature context or creature doesn't match - only colorless
        return ColorFlags(ColorFlags::COLORLESS);
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
            return ColorFlags(ColorFlags::GREEN | ColorFlags::BLACK);
        }
        return ColorFlags(ColorFlags::GREEN);
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
            return ColorFlags(ColorFlags::BLUE | ColorFlags::BLACK);
        }
        return ColorFlags(ColorFlags::BLUE);
    }

    // Handle Multiversal Passage - produces chosen color
    if land.base.name == "Multiversal Passage" {
        if let Some(chosen_type) = &permanent.chosen_basic_type {
            if let Ok(color) = parse_mana_color(chosen_type) {
                let mut flags = ColorFlags::new();
                flags.insert(color);
                return flags;
            }
        }
    }

    // Handle Starting Town - produces C for free, or any color for 1 life
    if land.base.name == "Starting Town" {
        if state.life > 1 {
            // Can pay 1 life for any color
            return ColorFlags(
                ColorFlags::COLORLESS | ColorFlags::WHITE | ColorFlags::BLUE |
                ColorFlags::BLACK | ColorFlags::RED | ColorFlags::GREEN
            );
        }
        // Only colorless if we can't afford the life
        return ColorFlags(ColorFlags::COLORLESS);
    }

    // Return land colors for other lands
    let mut flags = ColorFlags::new();
    for color in &land.colors {
        flags.insert(*color);
    }
    flags
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
/// This uses the same scarcity-based matching algorithm as tap_lands_for_cost
/// to ensure consistency between "can I cast?" and "actually cast".
pub fn can_afford_cost(
    cost: &ManaCost,
    state: &GameState,
    for_creature: Option<&CreatureCard>,
) -> bool {
    // Collect all land info (same as tap_lands_for_cost)
    let land_info: Vec<(usize, ColorFlags)> = state.battlefield.permanents()
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

    // Quick check: do we have enough total mana?
    let total_cost = cost.white + cost.blue + cost.black + cost.red + cost.green + cost.colorless + cost.generic;
    if (land_info.len() as u32) < total_cost {
        return false;
    }

    // Track which lands are "used" in our simulated assignment
    let mut used_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // Build list of (color, amount) pairs, only for colors we need
    let mut colors_to_pay: Vec<(ManaColor, u32)> = Vec::new();
    if cost.white > 0 { colors_to_pay.push((ManaColor::White, cost.white)); }
    if cost.blue > 0 { colors_to_pay.push((ManaColor::Blue, cost.blue)); }
    if cost.black > 0 { colors_to_pay.push((ManaColor::Black, cost.black)); }
    if cost.red > 0 { colors_to_pay.push((ManaColor::Red, cost.red)); }
    if cost.green > 0 { colors_to_pay.push((ManaColor::Green, cost.green)); }
    if cost.colorless > 0 { colors_to_pay.push((ManaColor::Colorless, cost.colorless)); }

    // Sort colors by scarcity: count how many lands can produce each color
    colors_to_pay.sort_by_key(|(color, _amount)| {
        land_info.iter().filter(|(_, colors)| colors.contains(*color)).count()
    });

    // Process colors in order of scarcity (same algorithm as tap_lands_for_cost)
    for (color, amount) in &colors_to_pay {
        let mut remaining = *amount;

        // Collect lands that can produce this color, sorted by flexibility
        let mut candidates: Vec<(usize, u32)> = land_info.iter()
            .filter(|(idx, colors)| !used_indices.contains(idx) && colors.contains(*color))
            .map(|(idx, colors)| (*idx, colors.count()))
            .collect();
        
        // Sort by flexibility: prefer lands with fewer colors (less flexible)
        candidates.sort_by_key(|(_, color_count)| *color_count);

        for (idx, _) in candidates {
            if remaining == 0 {
                break;
            }
            used_indices.insert(idx);
            remaining -= 1;
        }

        if remaining > 0 {
            return false;
        }
    }

    // Check if we can pay generic with remaining lands
    let generic_remaining = cost.generic;
    let available_for_generic = land_info.iter()
        .filter(|(idx, _)| !used_indices.contains(idx))
        .count() as u32;
    
    if available_for_generic < generic_remaining {
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
/// 
/// Strategy: Process colors in order of SCARCITY (fewest available lands first).
/// This ensures we don't "waste" flexible lands on colors that have many options.
/// When picking a land for a color, prefer lands with fewer total colors (less flexible).
pub fn tap_lands_for_cost(
    cost: &ManaCost,
    state: &mut GameState,
    for_creature: Option<&CreatureCard>,
) -> bool {
    // Collect all land info FIRST (before any mutations)
    // Each entry is (index, colors_this_land_produces as bitflags)
    let land_info: Vec<(usize, ColorFlags)> = state.battlefield.permanents()
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

    // Quick check: do we have enough total mana?
    let total_cost = cost.white + cost.blue + cost.black + cost.red + cost.green + cost.colorless + cost.generic;
    if (land_info.len() as u32) < total_cost {
        return false;
    }

    // Track which lands we'll tap (by index)
    let mut lands_to_tap: Vec<(usize, char)> = Vec::new();
    let mut used_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // Build list of (color, amount) pairs, only for colors we need
    let mut colors_to_pay: Vec<(ManaColor, u32)> = Vec::new();
    if cost.white > 0 { colors_to_pay.push((ManaColor::White, cost.white)); }
    if cost.blue > 0 { colors_to_pay.push((ManaColor::Blue, cost.blue)); }
    if cost.black > 0 { colors_to_pay.push((ManaColor::Black, cost.black)); }
    if cost.red > 0 { colors_to_pay.push((ManaColor::Red, cost.red)); }
    if cost.green > 0 { colors_to_pay.push((ManaColor::Green, cost.green)); }
    if cost.colorless > 0 { colors_to_pay.push((ManaColor::Colorless, cost.colorless)); }

    // Sort colors by scarcity: count how many lands can produce each color
    colors_to_pay.sort_by_key(|(color, _amount)| {
        land_info.iter().filter(|(_, colors)| colors.contains(*color)).count()
    });

    // Process colors in order of scarcity
    for (color, amount) in &colors_to_pay {
        let mut remaining = *amount;

        // Collect lands that can produce this color, sorted by flexibility (fewer colors = less flexible = use first)
        let mut candidates: Vec<(usize, u32)> = land_info.iter()
            .filter(|(idx, colors)| !used_indices.contains(idx) && colors.contains(*color))
            .map(|(idx, colors)| (*idx, colors.count()))
            .collect();
        
        // Sort by flexibility: prefer lands with fewer colors (less flexible)
        candidates.sort_by_key(|(_, color_count)| *color_count);

        for (idx, _) in candidates {
            if remaining == 0 {
                break;
            }
            lands_to_tap.push((idx, color.to_char()));
            used_indices.insert(idx);
            remaining -= 1;
        }

        if remaining > 0 {
            return false;
        }
    }

    // Pay generic with remaining untapped lands (prefer least flexible)
    let mut generic_remaining = cost.generic;
    let mut generic_candidates: Vec<(usize, u32)> = land_info.iter()
        .filter(|(idx, _)| !used_indices.contains(idx))
        .map(|(idx, colors)| (*idx, colors.count()))
        .collect();
    generic_candidates.sort_by_key(|(_, color_count)| *color_count);

    for (idx, _) in generic_candidates {
        if generic_remaining == 0 {
            break;
        }
        if let Some((_, colors)) = land_info.iter().find(|(i, _)| *i == idx) {
            if let Some(first) = colors.first_color() {
                lands_to_tap.push((idx, first.to_char()));
                used_indices.insert(idx);
                generic_remaining -= 1;
            }
        }
    }

    if generic_remaining > 0 {
        return false;
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

