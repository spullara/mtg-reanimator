use crate::card::{Card, CardType, LandCard, LandSubtype, ManaColor};
use crate::game::mana;
use crate::game::state::GameState;
use std::collections::HashSet;

/// Decision engine for MTG Reanimator AI
pub struct DecisionEngine;

impl DecisionEngine {
    /// Choose which land to play - matches TypeScript's sophisticated logic
    pub fn choose_land_to_play(hand: &[Card], state: &GameState) -> Option<usize> {
        let lands: Vec<(usize, &Card)> = hand
            .iter()
            .enumerate()
            .filter_map(|(idx, card)| {
                if matches!(card, Card::Land(_)) {
                    Some((idx, card))
                } else {
                    None
                }
            })
            .collect();

        if lands.is_empty() {
            return None;
        }

        let mut mana_available = 0;
        let mut colors_available = HashSet::new();
        for perm in state.battlefield.permanents() {
            if matches!(perm.card, Card::Land(_)) && !perm.tapped {
                mana_available += 1;
                // Use can_tap_for_mana to correctly evaluate conditional lands (Verge, Cavern, etc.)
                let land_colors = mana::can_tap_for_mana(perm, state, None);
                if land_colors.has_white() { colors_available.insert(ManaColor::White); }
                if land_colors.has_blue() { colors_available.insert(ManaColor::Blue); }
                if land_colors.has_black() { colors_available.insert(ManaColor::Black); }
                if land_colors.has_red() { colors_available.insert(ManaColor::Red); }
                if land_colors.has_green() { colors_available.insert(ManaColor::Green); }
            }
        }

        let mana_after_land_drop = mana_available + 1;
        let spells_in_hand: Vec<&Card> = hand.iter().filter(|c| !matches!(c, Card::Land(_))).collect();

        let mut missing_colors = HashSet::new();
        for spell in &spells_in_hand {
            Self::add_missing_colors(spell, &colors_available, &mut missing_colors);
        }

        let enters_tapped = |land: &LandCard| -> bool {
            match land.subtype {
                LandSubtype::Fastland => {
                    state.battlefield.permanents().iter().filter(|p| matches!(p.card, Card::Land(_))).count() >= 3
                }
                LandSubtype::Town => state.turn > 3,
                _ => land.enters_tapped,
            }
        };

        let provides_missing = |land: &LandCard| land.colors.iter().any(|c| missing_colors.contains(c));

        let can_cast_this_turn = |land: &LandCard| -> bool {
            if enters_tapped(land) { return false; }
            let mut colors_after = colors_available.clone();
            for color in &land.colors { colors_after.insert(*color); }
            spells_in_hand.iter().any(|spell| {
                spell.mana_value() <= mana_after_land_drop && Self::has_colors(spell, &colors_after)
            })
        };

        let mut sorted = lands.clone();
        sorted.sort_by(|a, b| {
            let (a_land, b_land) = match (a.1, b.1) {
                (Card::Land(al), Card::Land(bl)) => (al, bl),
                _ => return std::cmp::Ordering::Equal,
            };
            Self::compare_lands(a_land, b_land, &enters_tapped, &provides_missing, &can_cast_this_turn)
        });

        sorted.first().map(|(idx, _)| *idx)
    }

    fn add_missing_colors(spell: &Card, available: &HashSet<ManaColor>, missing: &mut HashSet<ManaColor>) {
        let cost = match spell {
            Card::Creature(c) => &c.base.mana_cost,
            Card::Enchantment(e) => &e.base.mana_cost,
            Card::Sorcery(s) => &s.base.mana_cost,
            Card::Instant(i) => &i.base.mana_cost,
            _ => return,
        };
        if cost.white > 0 && !available.contains(&ManaColor::White) { missing.insert(ManaColor::White); }
        if cost.blue > 0 && !available.contains(&ManaColor::Blue) { missing.insert(ManaColor::Blue); }
        if cost.black > 0 && !available.contains(&ManaColor::Black) { missing.insert(ManaColor::Black); }
        if cost.red > 0 && !available.contains(&ManaColor::Red) { missing.insert(ManaColor::Red); }
        if cost.green > 0 && !available.contains(&ManaColor::Green) { missing.insert(ManaColor::Green); }
    }

    fn has_colors(spell: &Card, colors: &HashSet<ManaColor>) -> bool {
        let cost = match spell {
            Card::Creature(c) => &c.base.mana_cost,
            Card::Enchantment(e) => &e.base.mana_cost,
            Card::Sorcery(s) => &s.base.mana_cost,
            Card::Instant(i) => &i.base.mana_cost,
            _ => return true,
        };
        (cost.white == 0 || colors.contains(&ManaColor::White))
            && (cost.blue == 0 || colors.contains(&ManaColor::Blue))
            && (cost.black == 0 || colors.contains(&ManaColor::Black))
            && (cost.red == 0 || colors.contains(&ManaColor::Red))
            && (cost.green == 0 || colors.contains(&ManaColor::Green))
    }

    fn compare_lands<F1, F2, F3>(a: &LandCard, b: &LandCard, enters_tapped: &F1, provides_missing: &F2, can_cast: &F3) -> std::cmp::Ordering
    where F1: Fn(&LandCard) -> bool, F2: Fn(&LandCard) -> bool, F3: Fn(&LandCard) -> bool,
    {
        let (a_cast, b_cast) = (can_cast(a), can_cast(b));
        if a_cast != b_cast { return if a_cast { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }; }

        if !a_cast && !b_cast {
            let (a_miss, b_miss) = (provides_missing(a), provides_missing(b));
            if a_miss != b_miss { return if a_miss { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }; }
            if a.has_surveil != b.has_surveil { return if a.has_surveil { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }; }
            let (a_tap, b_tap) = (enters_tapped(a), enters_tapped(b));
            if a_tap != b_tap { return if a_tap { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }; }
            return std::cmp::Ordering::Equal;
        }

        if a.has_surveil != b.has_surveil { return if a.has_surveil { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }; }
        b.colors.len().cmp(&a.colors.len())
    }

    /// Select the best card from a milled set based on game state priorities
    /// NEVER returns Bringer or Terror - they must stay in graveyard for reanimation
    pub fn select_best_from_mill<'a>(cards: &'a [Card], state: &GameState) -> Option<&'a Card> {
        if cards.is_empty() { return None; }

        let has_spider_in_hand = state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man");
        let has_bringer_in_hand = state.hand.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
        let land_count = state.battlefield.permanents().iter().filter(|p| matches!(p.card, Card::Land(_))).count();
        let lands_in_hand = state.hand.cards().iter().filter(|c| matches!(c, Card::Land(_))).count();

        // Priority 1: Superior Spider-Man (unless we already have one)
        if let Some(card) = cards.iter().find(|c| c.name() == "Superior Spider-Man" && !has_spider_in_hand) {
            return Some(card);
        }
        // Priority 2: Kiora if Bringer is stuck in hand
        if has_bringer_in_hand {
            if let Some(card) = cards.iter().find(|c| c.name() == "Kiora, the Rising Tide") { return Some(card); }
        }
        // Priority 3: Land if desperate
        if land_count <= 1 && lands_in_hand == 0 {
            if let Some(land) = cards.iter().find(|c| matches!(c, Card::Land(_))) { return Some(land); }
        }
        // Priority 4: Mill enablers
        if let Some(e) = cards.iter().find(|c| matches!(c, Card::Creature(_)) &&
            (c.name() == "Town Greeter" || c.name() == "Overlord of the Balemurk" || c.name() == "Kiora, the Rising Tide")) {
            return Some(e);
        }
        // Priority 5: Land if < 4 lands
        if land_count < 4 {
            if let Some(land) = cards.iter().find(|c| matches!(c, Card::Land(_))) { return Some(land); }
        }
        // Priority 6: Non-combo creature
        if let Some(c) = cards.iter().find(|c| matches!(c, Card::Creature(_)) && c.name() != "Bringer of the Last Gift" && c.name() != "Terror of the Peaks") {
            return Some(c);
        }
        // Priority 7: Any permanent except combo pieces
        cards.iter().find(|c| !matches!(c, Card::Instant(_) | Card::Sorcery(_)) && c.name() != "Bringer of the Last Gift" && c.name() != "Terror of the Peaks")
    }

    /// Choose which card to return from mill
    pub fn choose_mill_return(milled: &[Card], _card_type: CardType) -> Option<usize> {
        // Priority 1: Spider-Man
        if let Some(idx) = milled.iter().position(|c| c.name() == "Superior Spider-Man") { return Some(idx); }
        // Priority 2: Kiora
        if let Some(idx) = milled.iter().position(|c| c.name() == "Kiora, the Rising Tide") { return Some(idx); }
        // Priority 3: Blue-producing lands
        let blue_lands = ["Watery Grave", "Undercity Sewers", "Gloomlake Verge", "Island"];
        for (idx, card) in milled.iter().enumerate() {
            if let Card::Land(land) = card {
                if blue_lands.contains(&land.base.name.as_str()) { return Some(idx); }
            }
        }
        // Priority 4: Non-basic lands
        for (idx, card) in milled.iter().enumerate() {
            if let Card::Land(land) = card {
                if land.subtype != LandSubtype::Basic { return Some(idx); }
            }
        }
        // Priority 5: Basic lands
        if let Some(idx) = milled.iter().position(|c| matches!(c, Card::Land(_))) { return Some(idx); }
        // Priority 6: Non-combo creatures
        milled.iter().position(|c| matches!(c, Card::Creature(_)) && c.name() != "Bringer of the Last Gift" && c.name() != "Terror of the Peaks")
    }
}
