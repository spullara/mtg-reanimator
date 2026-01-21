/// Check if we can afford a mana cost given the current game state
/// This uses backtracking to properly handle multi-color lands
pub fn can_afford_cost(
    cost: &ManaCost,
    state: &GameState,
    for_creature: Option<&CreatureCard>,
) -> bool {
    // Collect land capabilities
    let land_colors: Vec<ColorFlags> = state
        .battlefield
        .permanents()
        .iter()
        .filter(|p| matches\!(p.card, Card::Land(_)) && \!p.tapped)
        .map(|p| can_tap_for_mana(p, state, for_creature))
        .filter(|c| \!c.is_empty())
        .collect();

    // Quick check: do we have enough total mana?
    let total_cost = cost.white + cost.blue + cost.black + cost.red + cost.green + cost.colorless + cost.generic;
    if (land_colors.len() as u32) < total_cost {
        return false;
    }

    // Build list of color requirements
    let mut requirements: Vec<ManaColor> = Vec::new();
    for _ in 0..cost.white { requirements.push(ManaColor::White); }
    for _ in 0..cost.blue { requirements.push(ManaColor::Blue); }
    for _ in 0..cost.black { requirements.push(ManaColor::Black); }
    for _ in 0..cost.red { requirements.push(ManaColor::Red); }
    for _ in 0..cost.green { requirements.push(ManaColor::Green); }
    for _ in 0..cost.colorless { requirements.push(ManaColor::Colorless); }
    // Generic mana can be paid with any color
    for _ in 0..cost.generic { requirements.push(ManaColor::Generic); }

    // Use backtracking to find a valid assignment
    let mut used = vec\![false; land_colors.len()];
    can_assign_colors(&requirements, 0, &land_colors, &mut used)
}

/// Recursive backtracking to check if color requirements can be assigned to lands
fn can_assign_colors(
    requirements: &[ManaColor],
    req_idx: usize,
    lands: &[ColorFlags],
    used: &mut [bool],
) -> bool {
    // Base case: all requirements satisfied
    if req_idx >= requirements.len() {
        return true;
    }

    let color = requirements[req_idx];

    // Try each unused land
    for (land_idx, land_colors) in lands.iter().enumerate() {
        if used[land_idx] {
            continue;
        }

        // Check if this land can produce the required color
        let can_produce = match color {
            ManaColor::Generic => true, // Any land can pay for generic
            _ => land_colors.contains(color),
        };

        if can_produce {
            used[land_idx] = true;
            if can_assign_colors(requirements, req_idx + 1, lands, used) {
                return true;
            }
            used[land_idx] = false;
        }
    }

    false
}
