use crate::card::{Card, CardDatabase, CardType, LandSubtype, ManaCost};
use crate::game::state::GameState;
use crate::game::zones::{CounterType, Permanent};
use crate::simulation::decisions::DecisionEngine;

/// Intern card names to static strings for zero-allocation copying
/// This uses a static lookup for known card names
pub fn intern_card_name(name: &str) -> &'static str {
    match name {
        "Bringer of the Last Gift" => "Bringer of the Last Gift",
        "Ardyn, the Usurper" => "Ardyn, the Usurper",
        "Terror of the Peaks" => "Terror of the Peaks",
        "Superior Spider-Man" => "Superior Spider-Man",
        "Kiora, the Rising Tide" => "Kiora, the Rising Tide",
        "Atraxa, Grand Unifier" => "Atraxa, Grand Unifier",
        "Overlord of the Balemurk" => "Overlord of the Balemurk",
        "Dredger's Insight" => "Dredger's Insight",
        "Town Greeter" => "Town Greeter",
        "Formidable Speaker" => "Formidable Speaker",
        "Dark Matter Manipulator" => "Dark Matter Manipulator",
        "Valgavoth, Terror Eater" => "Valgavoth, Terror Eater",
        "Marang River Regent" => "Marang River Regent",
        "Deceit" => "Deceit",
        // Fallback: leak the string to get a static reference
        // This is acceptable because card names are a small, fixed set
        _ => Box::leak(name.to_string().into_boxed_str()),
    }
}

/// Check if a creature has impending counters (enters as enchantment)
pub fn has_impending(card: &Card) -> bool {
    match card {
        Card::Creature(c) => c.impending_counters.is_some(),
        _ => false,
    }
}

/// Get impending counter count for a creature
pub fn get_impending_counters(card: &Card) -> u32 {
    match card {
        Card::Creature(c) => c.impending_counters.unwrap_or(0),
        _ => 0,
    }
}

/// Play a land from hand to battlefield with proper tapping logic
pub fn play_land(state: &mut GameState, card: &Card, verbose: bool) -> Result<(), String> {
    let land = match card {
        Card::Land(l) => l,
        _ => return Err("Not a land card".to_string()),
    };

    // Determine if land enters tapped
    let mut enters_tapped = land.enters_tapped;

    // Conditional tapping logic based on land subtype
    match land.subtype {
        LandSubtype::Shock => {
            // Shock lands can pay 2 life to enter untapped
            // For now, simplified: always enter untapped (decision logic in simulation)
            enters_tapped = false;
        }
        LandSubtype::Fastland => {
            // Enter untapped if you control 2 or fewer other lands
            let land_count = state
                .battlefield
                .permanents()
                .iter()
                .filter(|p| matches!(p.card, Card::Land(_)))
                .count();
            enters_tapped = land_count >= 3;
        }
        LandSubtype::Town => {
            // Starting Town: enters untapped on turns 1-3, tapped on turn 4+
            enters_tapped = state.turn > 3;
        }
        LandSubtype::Slowland => {
            // Shipwreck Marsh: enters tapped unless you control 2+ other lands
            let land_count = state
                .battlefield
                .permanents()
                .iter()
                .filter(|p| matches!(p.card, Card::Land(_)))
                .count();
            enters_tapped = land_count < 2;
        }
        LandSubtype::Utility => {
            // Verge lands (Wastewood Verge, Gloomlake Verge): simplified to always enter untapped
            // Other utility lands (Restless Cottage, Cavern, Passage): use card definition
            let is_verge = land.base.name.ends_with("Verge");
            if is_verge {
                enters_tapped = false;
            }
            // Otherwise use the card's enters_tapped value
        }
        _ => {} // Basic, Surveil use enters_tapped from card definition
    }

    let mut permanent = Permanent::new(card.clone(), state.turn);
    permanent.tapped = enters_tapped;

    // Handle Cavern of Souls - choose creature type
    if land.base.name == "Cavern of Souls" {
        let chosen_type = choose_cavern_type(state);
        if verbose {
            println!("    (Cavern set to: {})", chosen_type);
        }
        permanent.chosen_type = Some(chosen_type);
    }

    // Handle Multiversal Passage - choose basic land type (color)
    if land.base.name == "Multiversal Passage" {
        let chosen_color = choose_passage_color(state);
        if verbose {
            println!("    (Passage set to: {})", chosen_color);
        }
        permanent.chosen_basic_type = Some(chosen_color);
    }

    // Handle surveil lands
    if land.has_surveil && land.surveil_amount > 0 {
        resolve_surveil(state, land.surveil_amount as usize, verbose);
    }

    state.battlefield.add_permanent(permanent);
    state.land_played_this_turn = true;

    Ok(())
}

/// Choose creature type for Cavern of Souls
/// Priority: Human (Spider-Man, Town Greeter) > Demon (Bringer) > Noble (Kiora) > Dragon (Terror) > Avatar (Overlord)
fn choose_cavern_type(state: &GameState) -> String {
    // Get creatures in hand
    let creatures_in_hand: Vec<&Card> = state
        .hand
        .cards()
        .iter()
        .filter(|c| matches!(c, Card::Creature(_)))
        .collect();

    // Check if we already have a Cavern with Human type
    let has_human_cavern = state
        .battlefield
        .permanents()
        .iter()
        .any(|p| {
            if let Card::Land(l) = &p.card {
                l.base.name == "Cavern of Souls" && p.chosen_type.as_deref() == Some("Human")
            } else {
                false
            }
        });

    // Check if we have another Cavern in hand
    let caverns_in_hand = state
        .hand
        .cards()
        .iter()
        .filter(|c| c.name() == "Cavern of Souls")
        .count();

    let has_kiora_in_hand = creatures_in_hand.iter().any(|c| c.name() == "Kiora, the Rising Tide");
    let has_bringer_or_terror_in_hand = creatures_in_hand
        .iter()
        .any(|c| c.name() == "Bringer of the Last Gift" || c.name() == "Terror of the Peaks");

    // Special case: If we have Kiora + Bringer/Terror in hand AND another Cavern coming,
    // set this one to Noble (cast Kiora first to discard Bringer/Terror)
    if !has_human_cavern && has_kiora_in_hand && has_bringer_or_terror_in_hand && caverns_in_hand >= 1 {
        return "Noble".to_string();
    }

    if has_human_cavern {
        // We already have Human covered, pick something else based on hand
        if creatures_in_hand.iter().any(|c| c.name() == "Bringer of the Last Gift") {
            return "Demon".to_string();
        } else if creatures_in_hand.iter().any(|c| c.name() == "Kiora, the Rising Tide") {
            return "Noble".to_string();
        } else if creatures_in_hand.iter().any(|c| c.name() == "Overlord of the Balemurk") {
            return "Avatar".to_string();
        } else if creatures_in_hand.iter().any(|c| c.name() == "Terror of the Peaks") {
            return "Dragon".to_string();
        } else {
            // No specific need, default to Demon (in case we draw Bringer)
            return "Demon".to_string();
        }
    }

    // First Cavern - default to Human (helps Spider-Man and Town Greeter)
    "Human".to_string()
}

/// Helper to get mana cost from any card
fn card_mana_cost(card: &Card) -> &ManaCost {
    match card {
        Card::Land(c) => &c.base.mana_cost,
        Card::Creature(c) => &c.base.mana_cost,
        Card::Instant(c) => &c.base.mana_cost,
        Card::Sorcery(c) => &c.base.mana_cost,
        Card::Enchantment(c) => &c.base.mana_cost,
        Card::Saga(c) => &c.base.mana_cost,
    }
}

/// Choose basic land type for Multiversal Passage.
/// Uses the mana the lands can really make (not their printed color lists) and
/// only the spells in hand that you could cast soon, so a Terror of the Peaks
/// that waits to be discarded does not turn Passage into a Mountain.
fn choose_passage_color(state: &GameState) -> String {
    let lands: Vec<&Permanent> = state.battlefield.permanents().iter()
        .filter(|p| matches!(p.card, Card::Land(_)))
        .collect();

    // Real sources of each color among the lands on the battlefield
    let (mut u_src, mut b_src, mut g_src, mut r_src) = (0u32, 0u32, 0u32, 0u32);
    for perm in &lands {
        let mut p = (*perm).clone();
        p.tapped = false;
        let f = crate::game::mana::can_tap_for_mana(&p, state, None);
        if f.has_blue() { u_src += 1; }
        if f.has_black() { b_src += 1; }
        if f.has_green() { g_src += 1; }
        if f.has_red() { r_src += 1; }
    }

    // Colored pips of the spells you could cast soon (mana value at most the
    // land count after this land drop plus one)
    let soon = lands.len() as u32 + 2;
    let (mut u_need, mut b_need, mut g_need, mut r_need) = (0u32, 0u32, 0u32, 0u32);
    for card in state.hand.cards() {
        if matches!(card, Card::Land(_)) || card.mana_value() > soon {
            continue;
        }
        let cost = card_mana_cost(card);
        u_need = u_need.max(cost.blue as u32);
        b_need = b_need.max(cost.black as u32);
        g_need = g_need.max(cost.green as u32);
        r_need = r_need.max(cost.red as u32);
    }

    // 1. A color that a spell needs and no land makes
    if g_need > 0 && g_src == 0 { return "G".to_string(); }
    if u_need > 0 && u_src == 0 { return "U".to_string(); }
    if b_need > 0 && b_src == 0 { return "B".to_string(); }
    if r_need > 0 && r_src == 0 { return "R".to_string(); }

    // 2. Turn on Gloomlake Verge's {B}: be a Swamp if a Gloomlake is on the
    //    battlefield or in hand and no Island or Swamp is on the battlefield
    let gloomlake = lands.iter().any(|p| p.card.name() == "Gloomlake Verge")
        || state.hand.cards().iter().any(|c| c.name() == "Gloomlake Verge");
    let has_island_or_swamp = lands.iter()
        .any(|p| crate::game::mana::basic_land_types(p).iter().any(|t| *t == 'U' || *t == 'B'));
    if gloomlake && !has_island_or_swamp {
        return "B".to_string();
    }

    // 3. The color with the biggest shortfall (pips needed minus sources)
    let u_gap = u_need as i64 - u_src as i64;
    let b_gap = b_need as i64 - b_src as i64;
    if u_gap > 0 || b_gap > 0 {
        return if u_gap > b_gap { "U".to_string() } else { "B".to_string() };
    }

    // 4. No shortfall: balance U and B sources (black first: Beseech needs BBB)
    if u_src < b_src { "U".to_string() } else { "B".to_string() }
}

/// Cast a creature, handling impending logic
pub fn cast_creature(
    state: &mut GameState,
    card: &Card,
    use_impending: bool,
) -> Result<(), String> {
    match card {
        Card::Creature(_) => {},
        _ => return Err("Not a creature card".to_string()),
    };

    let mut permanent = Permanent::new(card.clone(), state.turn);

    // Handle impending creatures
    if use_impending && has_impending(card) {
        let counters = get_impending_counters(card);
        permanent.add_counter(CounterType::Time, counters);
    }

    state.battlefield.add_permanent(permanent);
    Ok(())
}

/// Cast a spell and resolve its effects
pub fn cast_spell(
    state: &mut GameState,
    card: &Card,
    db: &CardDatabase,
    verbose: bool,
    rng: &mut crate::rng::GameRng,
) -> Result<(), String> {
    match card {
        Card::Instant(spell) | Card::Sorcery(spell) => {
            // Process instant/sorcery abilities
            for ability in &spell.abilities {
                match ability.as_str() {
                    "mill_4_return_permanent" => {
                        // Cache Grab: mill 4, return permanent to hand
                        let milled = state.library.mill(4);
                        let mut milled_cards: Vec<Card> = Vec::new();
                        for card in milled {
                            milled_cards.push(card);
                        }

                        if verbose {
                            let names: Vec<&str> = milled_cards.iter().map(|c| c.name()).collect();
                            println!("    Mill 4: {}", names.join(", "));
                        }

                        // Filter to permanents only (not instant/sorcery)
                        let permanents: Vec<&Card> = milled_cards.iter()
                            .filter(|c| !matches!(c, Card::Instant(_) | Card::Sorcery(_)))
                            .collect();

                        // Choose best card to return using decision engine
                        let selected = if !permanents.is_empty() {
                            DecisionEngine::select_best_from_mill(&milled_cards, state)
                        } else {
                            None
                        };

                        // Return selected card to hand, rest to graveyard
                        let mut selected_name = selected.map(|c| c.name().to_string());
                        for card in milled_cards {
                            if Some(card.name().to_string()) == selected_name {
                                if verbose {
                                    println!("    -> Returned to hand: {}", card.name());
                                }
                                state.hand.add_card(card);
                                // Clear selected_name so we only return one copy
                                selected_name = None;
                            } else {
                                state.graveyard.add_card(card);
                            }
                        }
                    }
                    "mill_3_draw_1" => {
                        // Theorix Charm (mill mode): mill three cards, then draw a card.
                        // The counter and -2/-2 modes do nothing in a goldfish game.
                        for card in state.library.mill(3) {
                            if verbose {
                                println!("    Milled: {}", card.name());
                            }
                            state.graveyard.add_card(card);
                        }
                        state.draw_card();
                    }
                    "discard_outlet_removal" => {
                        // Bitter Triumph: additional cost is discard a card or pay 3 life.
                        // Use it as a discard outlet (bin a reanimation target), or pay
                        // the cost with a blank / 3 life when we only want the self-kill.
                        // Never pay life if it would put us at 3 or less;
                        // discard the least bad card instead
                        let discard_idx = DecisionEngine::reanimation_target_in_hand(state)
                            .or_else(|| DecisionEngine::choose_blank_discard(state))
                            .or_else(|| {
                                if state.life <= 6 { DecisionEngine::choose_discard(state) } else { None }
                            });
                        if let Some(idx) = discard_idx {
                            if let Some(discarded) = state.hand.remove_card(idx) {
                                if verbose {
                                    println!("    Bitter Triumph: discarded {}", discarded.name());
                                }
                                state.graveyard.add_card(discarded);
                            }
                        } else {
                            state.life -= 3;
                            if verbose {
                                println!("    Bitter Triumph: paid 3 life");
                            }
                        }

                        // It may destroy one of OUR creatures to put it into the
                        // graveyard, when the combo needs a creature there
                        if let Some(idx) = bitter_triumph_self_kill_target(state) {
                            if let Some(perm) = state.battlefield.remove_permanent(idx) {
                                if verbose {
                                    println!("    Bitter Triumph: destroyed own {} (graveyard fodder)", perm.card.name());
                                }
                                state.graveyard.add_card(perm.card);
                            }
                        }
                    }
                    "bargain_tutor" => {
                        // Beseech the Mirror
                        resolve_beseech(state, db, rng, verbose);
                    }
                    "search_land_or_creature_with_evidence" => {
                        // Analyze the Pollen: evidence 8 (total mana value), search for creature/land
                        // NEVER exile: Terror, Bringer (combo pieces), lands (MV 0, don't help)
                        let never_exile = ["Terror of the Peaks", "Bringer of the Last Gift"];

                        // Collect exilable cards with their indices and info
                        let exilable_cards: Vec<(usize, String, i32, &Card)> = state.graveyard.cards()
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| {
                                !matches!(c, Card::Land(_)) && !never_exile.contains(&c.name())
                            })
                            .map(|(i, c)| (i, c.name().to_string(), c.mana_value() as i32, c))
                            .collect();

                        // Calculate total exilable MV
                        let exilable_mv: i32 = exilable_cards.iter().map(|(_, _, mv, _)| mv).sum();
                        let can_collect_evidence = exilable_mv >= 8;

                        if can_collect_evidence {
                            // Sort by what we want to exile
                            // Priority: Spells > Enchantments > Creatures (minimize creature exile)
                            let mut sorted_exilable = exilable_cards.clone();
                            sorted_exilable.sort_by(|a, b| {
                                let type_order = |c: &Card| -> i32 {
                                    match c {
                                        Card::Instant(_) | Card::Sorcery(_) => 0,
                                        Card::Enchantment(_) | Card::Saga(_) => 1,
                                        Card::Creature(_) => 2,
                                        _ => 3,
                                    }
                                };
                                let order_diff = type_order(a.3).cmp(&type_order(b.3));
                                if order_diff != std::cmp::Ordering::Equal {
                                    return order_diff;
                                }
                                // Within same type, prefer higher MV to reach 8 faster
                                b.2.cmp(&a.2)
                            });

                            // Collect evidence - exile cards totaling 8+ MV
                            let mut evidence_mv = 0;
                            let mut to_exile: Vec<(usize, String)> = Vec::new();

                            for (idx, name, mv, _) in &sorted_exilable {
                                if evidence_mv >= 8 {
                                    break;
                                }
                                to_exile.push((*idx, name.clone()));
                                evidence_mv += mv;
                            }

                            // Sort indices in reverse order so we can remove from highest to lowest
                            to_exile.sort_by(|a, b| b.0.cmp(&a.0));

                            let exiled_names: Vec<String> = to_exile.iter().map(|(_, n)| n.clone()).collect();

                            for (idx, _) in &to_exile {
                                if let Some(card) = state.graveyard.remove_card(*idx) {
                                    state.add_to_exile(card);
                                }
                            }

                            if verbose {
                                println!("    Evidence collected ({} MV exiled: {})",
                                    evidence_mv, exiled_names.join(", "));
                            }

                            // Search for creature or land
                            // Priority: Spider-Man (if needed) > Kiora > land
                            let has_spider_man = state.hand.cards().iter()
                                .any(|c| c.name() == "Superior Spider-Man");
                            let has_bringer_in_gy = state.graveyard.cards().iter()
                                .any(|c| c.name() == "Bringer of the Last Gift");

                            let mut found_idx: Option<usize> = None;

                            // Search for Spider-Man if we need it
                            if !has_spider_man && has_bringer_in_gy {
                                for (i, card) in state.library.cards().iter().enumerate() {
                                    if card.name() == "Superior Spider-Man" {
                                        found_idx = Some(i);
                                        break;
                                    }
                                }
                            }

                            // Search for Kiora
                            if found_idx.is_none() {
                                for (i, card) in state.library.cards().iter().enumerate() {
                                    if card.name() == "Kiora, the Rising Tide" {
                                        found_idx = Some(i);
                                        break;
                                    }
                                }
                            }

                            // Search for a land
                            if found_idx.is_none() {
                                for (i, card) in state.library.cards().iter().enumerate() {
                                    if matches!(card, Card::Land(_)) {
                                        found_idx = Some(i);
                                        break;
                                    }
                                }
                            }

                            if let Some(idx) = found_idx {
                                let library_cards = state.library.cards_mut();
                                if idx < library_cards.len() {
                                    let target = library_cards.remove(idx);
                                    if verbose {
                                        println!("    -> Searched for: {}", target.name());
                                    }
                                    state.hand.add_card(target);
                                    // Shuffle library with deterministic RNG
                                    state.library.shuffle(rng);
                                }
                            }
                        } else {
                            // No evidence - just search for basic land
                            let graveyard_mv: u32 = state.graveyard.cards().iter()
                                .map(|c| c.mana_value())
                                .sum();
                            if verbose {
                                println!("    No evidence (graveyard MV: {}/8)", graveyard_mv);
                            }

                            // Find a basic land in library
                            let mut found_idx: Option<usize> = None;
                            for (i, card) in state.library.cards().iter().enumerate() {
                                if let Card::Land(land) = card {
                                    if land.subtype == LandSubtype::Basic {
                                        found_idx = Some(i);
                                        break;
                                    }
                                }
                            }

                            if let Some(idx) = found_idx {
                                let library_cards = state.library.cards_mut();
                                if idx < library_cards.len() {
                                    let target = library_cards.remove(idx);
                                    if verbose {
                                        println!("    -> Searched for basic land: {}", target.name());
                                    }
                                    state.hand.add_card(target);
                                    // Shuffle library with deterministic RNG
                                    state.library.shuffle(rng);
                                }
                            } else {
                                if verbose {
                                    println!("    -> No basic land found in library");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Instant/Sorcery goes to graveyard after resolution
            state.graveyard.add_card(card.clone());
            Ok(())
        }
        Card::Enchantment(spell) => {
            // Add enchantment to battlefield
            let permanent = Permanent::new(card.clone(), state.turn);
            state.battlefield.add_permanent(permanent);

            // Process enchantment abilities
            for ability in &spell.abilities {
                match ability.as_str() {
                    "etb_mill_4_return_artifact_creature_land" => {
                        // Dredger's Insight: mill 4, return artifact/creature/land to hand
                        let milled = state.library.mill(4);
                        let mut milled_cards = Vec::new();
                        for card in milled {
                            milled_cards.push(card);
                        }

                        if verbose {
                            let names: Vec<&str> = milled_cards.iter().map(|c| c.name()).collect();
                            println!("    Mill 4: {}", names.join(", "));
                        }

                        // Choose which card to return (prioritize Spider-Man, then Kiora, then lands)
                        if let Some(idx) = DecisionEngine::choose_mill_return(&milled_cards, CardType::Creature) {
                            let card_to_return = milled_cards.remove(idx);
                            if verbose {
                                println!("    -> Returned to hand: {}", card_to_return.name());
                            }
                            state.hand.add_card(card_to_return);
                        }

                        // Rest go to graveyard
                        for card in milled_cards {
                            state.graveyard.add_card(card);
                        }
                    }
                    "graveyard_leave_lifegain" => {
                        // Dredger's Insight: gain life when leaving graveyard
                        // This is a triggered ability, handled elsewhere
                    }
                    "etb_draw_2_discard_1" => {
                        // Roiling Dragonstorm: draw two cards, then discard a card
                        resolve_draw_n_discard_n(state, 2, 1, verbose);
                    }
                    "etb_drain_2_gain_2_surveil_2" => {
                        // Corroding Dragonstorm: each opponent loses 2 life and
                        // you gain 2 life. Surveil 2.
                        state.opponent_life -= 2;
                        state.life += 2;
                        if verbose {
                            println!("    Corroding Dragonstorm: opponent loses 2, gain 2 life");
                        }
                        resolve_surveil(state, 2, verbose);
                    }
                    "dragon_bounce_home" => {
                        // Return-to-hand trigger: handled when a Dragon enters
                        // (return_dragonstorms_to_hand), nothing to do on cast
                    }
                    "etb_mill_2_draw_2_discard_2" => {
                        // Does Machines: mill 2, draw 2, then discard 2
                        resolve_mill_n(state, 2, verbose);

                        let hand_before = state.hand.size();
                        state.draw_card();
                        state.draw_card();
                        if verbose {
                            let drawn: Vec<&str> = state.hand.cards().iter()
                                .skip(hand_before)
                                .map(|c| c.name())
                                .collect();
                            println!("    Does Machines: drew {}", drawn.join(", "));
                        }

                        let mut discarded: Vec<String> = Vec::new();
                        for _ in 0..2 {
                            if let Some(idx) = DecisionEngine::choose_discard(state) {
                                if let Some(card) = state.hand.remove_card(idx) {
                                    discarded.push(card.name().to_string());
                                    state.graveyard.add_card(card);
                                }
                            }
                        }
                        if verbose && !discarded.is_empty() {
                            println!("    Does Machines: discarded {}", discarded.join(", "));
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        Card::Saga(saga) => {
            // Add saga to battlefield with 1 lore counter
            let saga_name = saga.base.name.clone();
            let mut permanent = Permanent::new(card.clone(), state.turn);
            permanent.add_counter(CounterType::Time, 1);
            state.battlefield.add_permanent(permanent);
            
            // Resolve Chapter I immediately
            resolve_saga_chapter(state, &saga_name, 1, verbose);
            
            Ok(())
        }
        _ => Err("Not a spell card".to_string()),
    }
}

/// Process enter-the-battlefield triggers for a creature (with verbose output)
pub fn process_etb_triggers_verbose(
    state: &mut GameState,
    permanent: &mut Permanent,
    _db: &CardDatabase,
    verbose: bool,
    rng: &mut crate::rng::GameRng,
) -> Result<(), String> {
    // Extract abilities before borrowing permanent mutably
    let abilities = match &permanent.card {
        Card::Creature(c) => c.abilities.clone(),
        _ => return Ok(()), // Not a creature
    };

    // Process abilities
    for ability in abilities {
        match ability.as_str() {
            "etb_mill_4_return_land" => {
                // Town Greeter: mill 4, may return land
                let milled = state.library.mill(4);
                let mut milled_cards = Vec::new();
                for card in milled {
                    milled_cards.push(card);
                }

                if verbose {
                    let mill_names: Vec<String> = milled_cards.iter().map(|c| c.name().to_string()).collect();
                    println!("    Mill 4: {}", mill_names.join(", "));
                }

                // Find the best land to return
                let mut best_land: Option<Card> = None;
                let mut best_land_idx: Option<usize> = None;

                for (idx, card) in milled_cards.iter().enumerate() {
                    if matches!(card, Card::Land(_)) {
                        // Prefer untapped lands, then multi-color lands
                        if let Some(ref current_best) = best_land {
                            let new_is_better = match (card, current_best) {
                                (Card::Land(new_land), Card::Land(current_land)) => {
                                    let new_tapped = new_land.enters_tapped;
                                    let current_tapped = current_land.enters_tapped;
                                    if new_tapped != current_tapped {
                                        !new_tapped // Prefer untapped
                                    } else {
                                        new_land.colors.len() > current_land.colors.len() // Prefer multi-color
                                    }
                                }
                                _ => false,
                            };
                            if new_is_better {
                                best_land = Some(card.clone());
                                best_land_idx = Some(idx);
                            }
                        } else {
                            best_land = Some(card.clone());
                            best_land_idx = Some(idx);
                        }
                    }
                }

                // Return the best land to hand, rest to graveyard
                for (idx, card) in milled_cards.into_iter().enumerate() {
                    if Some(idx) == best_land_idx {
                        if verbose {
                            println!("    -> Returned to hand: {}", card.name());
                        }
                        state.hand.add_card(card);
                    } else {
                        state.graveyard.add_card(card);
                    }
                }
            }
            "etb_draw_2_discard_2" => {
                // Kiora: draw 2, discard 2 - use the proper priority logic
                resolve_kiora_etb(state, verbose);
            }
            "etb_mill_3" => {
                // Dark Matter Manipulator: mill 3
                resolve_mill_n(state, 3, verbose);
            }
            "etb_discard_1_draw_1" => {
                // Fear of Missing Out: discard 1, then draw 1
                resolve_fear_of_missing_out_etb(state, verbose);
            }
            "etb_surveil_2" => {
                // Wreckage Wickerfolk / Yuriko (always chooses this mode): surveil 2
                resolve_surveil(state, 2, verbose);
            }
            "etb_draw_1_discard_1" => {
                // Bellowing Crier: draw a card, then discard a card
                resolve_draw_n_discard_n(state, 1, 1, verbose);
            }
            "etb_bounce_up_to_two" => {
                // Marang River Regent: a Dragon entered, so every Dragonstorm
                // returns to hand first (it needs no bounce slot), then return
                // up to two other nonland permanents to hand (bounce our own
                // ETB-value permanents to recast them)
                return_dragonstorms_to_hand(state, verbose);
                resolve_bounce_recast_targets(state, 2, verbose);
            }
            "etb_discard_tutor_creature" => {
                // Formidable Speaker: may discard a card to tutor a creature
                resolve_formidable_speaker_etb(state, rng, verbose);
            }
            "impending_5" => {
                // Impending counters are already added by cast_creature when use_impending=true
                // This ability is just a marker - no action needed here
            }
            "etb_damage_trigger" => {
                // Terror of the Peaks: damage trigger (setup, actual damage on creature ETB)
                // This is a triggered ability that fires when other creatures enter
                // Stored for later trigger resolution
            }
            "etb_mass_reanimate" => {
                // Bringer of the Last Gift: mass reanimate
                // Return all creature cards from graveyard to battlefield
                let graveyard_cards = state.graveyard.cards().to_vec();
                for card in graveyard_cards {
                    if matches!(card, Card::Creature(_)) {
                        let perm = Permanent::new(card.clone(), state.turn);
                        state.battlefield.add_permanent(perm);
                    }
                }
                // Clear graveyard of creatures
                state.graveyard.clear_creatures();
            }
            "etb_or_attack_mill_4_return" => {
                // Overlord of the Balemurk: mill 4, may return non-Avatar creature or land
                // BUT we usually DON'T want to return creatures - we want them in graveyard for reanimate!
                let milled = state.library.mill(4);

                if verbose {
                    let mill_names: Vec<String> = milled.iter().map(|c| c.name().to_string()).collect();
                    println!("    Mill 4: {}", mill_names.join(", "));
                }

                // Check game state for selection logic
                let has_bringer_in_gy = state.graveyard.cards().iter()
                    .any(|c| c.name() == "Bringer of the Last Gift");
                let has_spider_in_hand = state.hand.cards().iter()
                    .any(|c| c.name() == "Superior Spider-Man");
                let has_bringer_in_hand = state.hand.cards().iter()
                    .any(|c| c.name() == "Bringer of the Last Gift");
                let land_count = state.battlefield.permanents().iter()
                    .filter(|p| matches!(p.card, Card::Land(_)))
                    .count();

                let mut selected_idx: Option<usize> = None;

                // Priority 1: Spider-Man if we need it for the combo
                if has_bringer_in_gy && !has_spider_in_hand {
                    for (idx, card) in milled.iter().enumerate() {
                        if card.name() == "Superior Spider-Man" {
                            selected_idx = Some(idx);
                            if verbose {
                                println!("    Overlord returns Superior Spider-Man (combo piece!)");
                            }
                            break;
                        }
                    }
                }

                // Priority 2: Kiora if Bringer is stuck in hand
                if selected_idx.is_none() && has_bringer_in_hand {
                    for (idx, card) in milled.iter().enumerate() {
                        if card.name() == "Kiora, the Rising Tide" {
                            selected_idx = Some(idx);
                            if verbose {
                                println!("    Overlord returns Kiora (need to discard Bringer from hand)");
                            }
                            break;
                        }
                    }
                }

                // Priority 3: Town Greeter if early game
                if selected_idx.is_none() && land_count < 4 {
                    for (idx, card) in milled.iter().enumerate() {
                        if card.name() == "Town Greeter" {
                            selected_idx = Some(idx);
                            if verbose {
                                println!("    Overlord returns Town Greeter (cheap enabler)");
                            }
                            break;
                        }
                    }
                }

                // Otherwise: DON'T return anything! Leave creatures in graveyard for reanimation
                if selected_idx.is_none() && verbose {
                    println!("    Overlord returns nothing (keeping creatures for reanimate)");
                }

                // Add cards to graveyard or hand
                for (idx, card) in milled.into_iter().enumerate() {
                    if Some(idx) == selected_idx {
                        state.hand.add_card(card);
                    } else {
                        state.graveyard.add_card(card);
                    }
                }
            }
            "mind_swap_copy" => {
                // Superior Spider-Man: copy creature from graveyard
                // Priority 1: Copy Bringer if in graveyard (THE COMBO!)
                // Priority 2: Copy Ardyn if in graveyard AND there are other creatures
                // Priority 3: If no Bringer/Ardyn but have another Spider-Man in hand,
                //             copy a mill creature to dig for Bringer

                let bringer_idx = state.graveyard.cards().iter()
                    .position(|c| c.name() == "Bringer of the Last Gift");

                if let Some(idx) = bringer_idx {
                    if verbose {
                        println!("    *** COMBO! Superior Spider-Man copies Bringer of the Last Gift! ***");
                    }

                    // Copy Bringer! (Spider-Man stays 4/4 but gains Bringer's types and triggers ETB)
                    permanent.is_copy_of = Some("Bringer of the Last Gift");

                    // Exile the copied card
                    if let Some(bringer) = state.graveyard.remove_card(idx) {
                        state.exile.add_card(bringer);
                    }

                    // Now trigger Bringer's ETB (mass reanimate!)
                    resolve_bringer_etb(state, rng, verbose);
                    return Ok(());
                }

                // Priority 2: Copy Ardyn if in graveyard AND there are other creatures
                // (Ardyn's Starscourge will create 5/5 Demon tokens from those creatures)
                let ardyn_idx = state.graveyard.cards().iter()
                    .position(|c| c.name() == "Ardyn, the Usurper");

                let other_creatures_count = state.graveyard.cards().iter()
                    .filter(|c| matches!(c, Card::Creature(_)) && c.name() != "Ardyn, the Usurper")
                    .count();

                if ardyn_idx.is_some() && other_creatures_count >= 1 {
                    let idx = ardyn_idx.unwrap();
                    if verbose {
                        println!("    *** Spider-Man copies Ardyn, the Usurper! ({} creatures for Starscourge) ***", other_creatures_count);
                    }

                    // Copy Ardyn (Spider-Man stays 4/4 but gains Demon type for haste and triggers Starscourge)
                    permanent.is_copy_of = Some("Ardyn, the Usurper");

                    // Exile Ardyn from graveyard
                    if let Some(ardyn) = state.graveyard.remove_card(idx) {
                        state.exile.add_card(ardyn);
                    }

                    // Note: Ardyn's Starscourge triggers at beginning of combat,
                    // not on ETB, so no trigger to resolve here
                    return Ok(());
                }

                // Priority 3: If no Bringer/Ardyn but have another Spider-Man in hand,
                // copy a mill creature to dig for Bringer
                let spider_man_in_hand = state.hand.cards().iter()
                    .filter(|c| c.name() == "Superior Spider-Man")
                    .count();

                if spider_man_in_hand >= 1 {
                    // We have another Spider-Man - copy a mill creature to dig for Bringer
                    // Priority: Overlord of the Balemurk > Dark Matter Manipulator > Kiora > Town Greeter
                    let mill_creature = state.graveyard.cards().iter()
                        .position(|c| c.name() == "Overlord of the Balemurk")
                        .or_else(|| state.graveyard.cards().iter()
                            .position(|c| c.name() == "Dark Matter Manipulator"))
                        .or_else(|| state.graveyard.cards().iter()
                            .position(|c| c.name() == "Kiora, the Rising Tide"))
                        .or_else(|| state.graveyard.cards().iter()
                            .position(|c| c.name() == "Town Greeter"));

                    if let Some(idx) = mill_creature {
                        let creature_name = state.graveyard.cards()[idx].name().to_string();
                        if verbose {
                            println!("    Spider-Man copies {} to dig for Bringer (have another Spider-Man in hand)", creature_name);
                        }

                        // Copy the mill creature (Spider-Man stays 4/4 but triggers the copied creature's ETB)
                        permanent.is_copy_of = Some(intern_card_name(&creature_name));

                        // Exile the copied card
                        if let Some(creature) = state.graveyard.remove_card(idx) {
                            state.exile.add_card(creature);
                        }

                        // Trigger the copied creature's ETB
                        match creature_name.as_str() {
                            "Overlord of the Balemurk" => {
                                // Mill 4, return a permanent
                                resolve_overlord_etb(state, verbose);
                            }
                            "Dark Matter Manipulator" => {
                                // Mill 3
                                resolve_mill_n(state, 3, verbose);
                            }
                            "Kiora, the Rising Tide" => {
                                // Draw 2, discard 2
                                resolve_kiora_etb(state, verbose);
                            }
                            "Town Greeter" => {
                                // Mill 4, return a land
                                resolve_town_greeter_etb(state, verbose);
                            }
                            _ => {}
                        }
                        return Ok(());
                    }
                }

                // Priority 4: Fallback copy targets - prefer the one that helps most:
                // Marang (bounce ETB) > Valgavoth (Demon, Ardyn haste) > Deceit
                let fallback_idx = state.graveyard.cards().iter()
                    .position(|c| c.name() == "Marang River Regent"
                        && !DecisionEngine::choose_bounce_targets(state, 1).is_empty())
                    .or_else(|| state.graveyard.cards().iter()
                        .position(|c| c.name() == "Valgavoth, Terror Eater"))
                    .or_else(|| state.graveyard.cards().iter()
                        .position(|c| c.name() == "Deceit"));

                if let Some(idx) = fallback_idx {
                    let creature_name = state.graveyard.cards()[idx].name().to_string();
                    if verbose {
                        println!("    Spider-Man copies {} (fallback target)", creature_name);
                    }
                    permanent.is_copy_of = Some(intern_card_name(&creature_name));
                    if let Some(creature) = state.graveyard.remove_card(idx) {
                        state.exile.add_card(creature);
                    }
                    if creature_name == "Marang River Regent" {
                        // The copy keeps Marang's types: a Dragon entered
                        return_dragonstorms_to_hand(state, verbose);
                        resolve_bounce_recast_targets(state, 2, verbose);
                    }
                } else if verbose {
                    println!("    Spider-Man enters as a 4/4 (no good copy target)");
                }
            }
            _ => {} // Other abilities handled elsewhere
        }
    }

    // A Dragon entering returns each Dragonstorm on the battlefield to hand
    // (card types of the copied card are kept, so copies count too)
    if permanent_is_dragon(permanent) {
        return_dragonstorms_to_hand(state, verbose);
    }

    Ok(())
}

/// Known Dragons for copy checks (a copy keeps the copied card's types)
const KNOWN_DRAGONS: &[&str] = &["Marang River Regent", "Terror of the Peaks"];

/// Check if a permanent is a Dragon (by its own types or as a copy of a Dragon)
fn permanent_is_dragon(permanent: &Permanent) -> bool {
    let card_is_dragon = match &permanent.card {
        Card::Creature(c) => c.creature_types.iter().any(|t| t == "Dragon"),
        _ => false,
    };
    let copy_is_dragon = permanent
        .is_copy_of
        .map(|name| KNOWN_DRAGONS.contains(&name))
        .unwrap_or(false);
    card_is_dragon || copy_is_dragon
}

/// "When a Dragon you control enters, return this enchantment to its owner's
/// hand." Move every Dragonstorm from the battlefield to hand so it can be
/// recast later for its ETB.
pub fn return_dragonstorms_to_hand(state: &mut GameState, verbose: bool) {
    loop {
        let idx = state.battlefield.permanents().iter().position(|p| {
            matches!(p.card.name(), "Roiling Dragonstorm" | "Corroding Dragonstorm")
                && p.is_copy_of.is_none()
        });
        let Some(idx) = idx else { break };
        if let Some(perm) = state.battlefield.remove_permanent(idx) {
            if verbose {
                println!("    {} returns to hand (a Dragon entered)", perm.card.name());
            }
            state.hand.add_card(perm.card);
        }
    }
}

/// Draw `draw` cards, then discard `discard` cards (Bellowing Crier's loot)
pub fn resolve_draw_n_discard_n(state: &mut GameState, draw: usize, discard: usize, verbose: bool) {
    let hand_before = state.hand.size();
    for _ in 0..draw {
        state.draw_card();
    }
    if verbose {
        let drawn: Vec<&str> = state.hand.cards().iter()
            .skip(hand_before)
            .map(|c| c.name())
            .collect();
        if !drawn.is_empty() {
            println!("    Drew: {}", drawn.join(", "));
        }
    }
    for _ in 0..discard {
        if let Some(idx) = DecisionEngine::choose_discard(state) {
            if let Some(card) = state.hand.remove_card(idx) {
                if verbose {
                    println!("    Discarded: {}", card.name());
                }
                state.graveyard.add_card(card);
            }
        }
    }
}

/// Resolve Bringer of the Last Gift ETB: sacrifice all other creatures, then mass reanimate
///
/// EXACT LOGIC FROM TYPESCRIPT resolveBringerETB:
/// 1. Sacrifice all other creatures (except impending ones with time counters)
/// 2. Return ALL creature cards from graveyard to battlefield
/// 3. Trigger Terror of the Peaks for each creature entering
pub fn resolve_bringer_etb(state: &mut GameState, rng: &mut crate::rng::GameRng, verbose: bool) {
    // Step 1: Sacrifice all other creatures (move to graveyard)
    // NOTE: Impending creatures (with time counters) are NOT creatures yet - they're enchantments!
    // NOTE: We need to find the Spider-Man that just entered (the one copying Bringer)
    // and exclude it from sacrifice. It's the last permanent added to the battlefield.
    let bringer_copy_idx = state.battlefield.permanents().len().saturating_sub(1);

    let mut to_sacrifice: Vec<usize> = Vec::new();

    for (idx, perm) in state.battlefield.permanents().iter().enumerate() {
        // Skip the Spider-Man that just entered (copying Bringer)
        if idx == bringer_copy_idx {
            continue;
        }
        // Skip non-creatures
        if !matches!(perm.card, Card::Creature(_)) {
            continue;
        }
        // Skip impending creatures (have time counters)
        if perm.get_counter(CounterType::Time) > 0 {
            if verbose {
                println!("    Impending survives: {} ({} counters)",
                    perm.card.name(), perm.get_counter(CounterType::Time));
            }
            continue;
        }
        to_sacrifice.push(idx);
    }

    if verbose && !to_sacrifice.is_empty() {
        let names: Vec<String> = to_sacrifice.iter()
            .map(|&idx| state.battlefield.permanents()[idx].card.name().to_string())
            .collect();
        println!("    Sacrifice: {}", names.join(", "));
    }

    // Remove sacrificed creatures and add to graveyard (in reverse order to preserve indices)
    for &idx in to_sacrifice.iter().rev() {
        if let Some(perm) = state.battlefield.remove_permanent(idx) {
            state.graveyard.add_card(perm.card);
        }
    }

    // Step 2: Return ALL creature cards from graveyard to battlefield
    let creatures_to_reanimate: Vec<Card> = state.graveyard.cards()
        .iter()
        .filter(|c| matches!(c, Card::Creature(_)))
        .cloned()
        .collect();

    if verbose && !creatures_to_reanimate.is_empty() {
        let names: Vec<String> = creatures_to_reanimate.iter()
            .map(|c| c.name().to_string())
            .collect();
        println!("    Reanimate: {}", names.join(", "));
    }

    // Handle Superior Spider-Man's copy choice BEFORE clearing graveyard
    // When reanimated, Spider-Man should copy Terror of the Peaks (not Bringer)
    // because Bringer already triggered the reanimate
    let spider_man_being_reanimated = creatures_to_reanimate.iter()
        .any(|c| c.name() == "Superior Spider-Man");

    let spider_man_copy_target: Option<&'static str> = if spider_man_being_reanimated {
        // Look for Terror of the Peaks in graveyard to copy
        // Note: Terror might also be in creatures_to_reanimate, but Spider-Man
        // copies from graveyard, so we check if Terror is there
        let terror_in_graveyard = state.graveyard.cards().iter()
            .any(|c| c.name() == "Terror of the Peaks");

        if terror_in_graveyard {
            if verbose {
                println!("    Superior Spider-Man (reanimated) copies Terror of the Peaks!");
            }
            // Remove Terror from graveyard and exile it
            if let Some(idx) = state.graveyard.cards().iter()
                .position(|c| c.name() == "Terror of the Peaks")
            {
                if let Some(terror) = state.graveyard.remove_card(idx) {
                    state.exile.add_card(terror);
                }
            }
            Some("Terror of the Peaks")
        } else {
            if verbose {
                println!("    Superior Spider-Man (reanimated) enters as a 4/4 (no Terror to copy)");
            }
            None
        }
    } else {
        None
    };

    // Remove remaining creatures from graveyard
    state.graveyard.clear_creatures();

    // Add to battlefield
    for creature in &creatures_to_reanimate {
        let mut perm = Permanent::new(creature.clone(), state.turn);

        // Apply Spider-Man's copy if this is Spider-Man
        if creature.name() == "Superior Spider-Man" {
            if let Some(copy_target) = spider_man_copy_target {
                perm.is_copy_of = Some(copy_target);
            }
        }

        state.battlefield.add_permanent(perm);
    }

    // Step 3: Resolve ETBs for reanimated creatures
    for creature in &creatures_to_reanimate {
        match creature.name() {
            "Kiora, the Rising Tide" => {
                resolve_kiora_etb(state, verbose);
            }
            "Town Greeter" => {
                resolve_town_greeter_etb(state, verbose);
            }
            "Overlord of the Balemurk" => {
                resolve_overlord_etb(state, verbose);
            }
            "Formidable Speaker" => {
                resolve_formidable_speaker_etb(state, rng, verbose);
            }
            "Dark Matter Manipulator" => {
                resolve_mill_n(state, 3, verbose);
            }
            "Fear of Missing Out" => {
                resolve_fear_of_missing_out_etb(state, verbose);
            }
            "Wreckage Wickerfolk" | "Yuriko, Hope from the Shadows" => {
                resolve_surveil(state, 2, verbose);
            }
            "Bellowing Crier" => {
                resolve_draw_n_discard_n(state, 1, 1, verbose);
            }
            "Marang River Regent" => {
                resolve_bounce_recast_targets(state, 2, verbose);
            }
            _ => {}
        }
    }

    // A Dragon entering (reanimated Marang/Terror, or Spider-Man copying
    // Terror) returns each Dragonstorm on the battlefield to hand
    let dragon_entered = creatures_to_reanimate.iter().any(|c| {
        matches!(c, Card::Creature(cr) if cr.creature_types.iter().any(|t| t == "Dragon"))
    }) || spider_man_copy_target == Some("Terror of the Peaks");
    if dragon_entered {
        return_dragonstorms_to_hand(state, verbose);
    }

    // Step 4: Resolve Terror triggers for each creature that entered
    // Note: If Spider-Man copied Terror, it now counts as a Terror for triggers!
    resolve_terror_triggers(state, &creatures_to_reanimate, verbose);

}

/// Resolve Terror of the Peaks triggers for creatures entering the battlefield
///
/// EXACT LOGIC FROM TYPESCRIPT resolveTerrorTriggers:
/// - Count Terrors on battlefield
/// - Each Terror triggers for each OTHER creature entering (not itself)
/// - Deal damage equal to creature's power for each Terror
fn resolve_terror_triggers(state: &mut GameState, entering: &[Card], verbose: bool) {
    // Count how many Terrors are on the battlefield
    let terror_count = state.battlefield.permanents().iter()
        .filter(|p| {
            p.card.name() == "Terror of the Peaks"
                || p.is_copy_of.as_deref() == Some("Terror of the Peaks")
        })
        .count() as i32;

    if terror_count == 0 {
        return;
    }

    // Each Terror triggers for each OTHER creature entering
    // (Terror doesn't trigger for itself)
    let mut total_damage = 0i32;

    for creature in entering {
        if creature.name() == "Terror of the Peaks" {
            continue; // Doesn't trigger for itself
        }

        if let Card::Creature(c) = creature {
            // Each Terror deals damage equal to the creature's power
            total_damage += c.power as i32 * terror_count;
        }
    }

    state.opponent_life -= total_damage;

    if verbose && total_damage > 0 {
        println!("  Terror triggers dealt {} damage! ({} Terror(s), {} creatures entered)",
            total_damage, terror_count, entering.len());
    }
}

/// Resolve surveil mechanic: look at top N cards and decide which go to graveyard
///
/// EXACT LOGIC FROM TYPESCRIPT:
/// - Check hasKioraInHand INSIDE the loop (it can change)
/// - Only remove from library if putting in graveyard
/// - If keeping on top, do NOT touch the library - leave card in place
pub fn resolve_surveil(state: &mut GameState, count: usize, verbose: bool) {
    let mut to_graveyard: Vec<String> = Vec::new();
    let mut to_top: Vec<String> = Vec::new();

    for _ in 0..count {
        // Check if library is empty
        if state.library.is_empty() {
            break;
        }

        // Peek at top card without removing it
        if let Some(top_card) = state.library.peek_top() {
            let card_name = top_card.name().to_string();

            // Decision: keep on top or put in graveyard?
            // Graveyard: Bringer, Terror, Overlord (want to reanimate these)
            // Also put Kiora if we already have one (for reanimation value)
            // Top: Spider-Man (MUST stay in hand!), lands, mill spells
            let has_kiora_in_hand = state.hand.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide");
            let put_in_graveyard = card_name == "Bringer of the Last Gift"
                || card_name == "Terror of the Peaks"
                || card_name == "Overlord of the Balemurk"
                || card_name == "Valgavoth, Terror Eater"
                || card_name == "Marang River Regent"
                || card_name == "Deceit"
                || (card_name == "Kiora, the Rising Tide" && has_kiora_in_hand)
                || card_name == "Town Greeter"; // Cheap 1/1, better to reanimate than draw

            if put_in_graveyard {
                // Remove from library and add to graveyard
                if let Some(card) = state.library.draw() {
                    state.graveyard.add_card(card);
                    to_graveyard.push(card_name);
                }
            } else {
                // Keep on top - do NOT touch the library
                to_top.push(card_name);
            }
        }
    }

    if verbose && (!to_graveyard.is_empty() || !to_top.is_empty()) {
        if !to_graveyard.is_empty() {
            println!("    Surveil -> graveyard: {}", to_graveyard.join(", "));
        }
        if !to_top.is_empty() {
            println!("    Surveil -> kept on top: {}", to_top.join(", "));
        }
    }
}

/// Resolve Overlord of the Balemurk ETB ability: mill 4, may return a permanent
/// Called when Spider-Man copies Overlord to dig for Bringer
pub fn resolve_overlord_etb(state: &mut GameState, verbose: bool) {
    let milled = state.library.mill(4);

    if verbose {
        let mill_names: Vec<String> = milled.iter().map(|c| c.name().to_string()).collect();
        println!("    Mill 4: {}", mill_names.join(", "));
    }

    // Check game state for selection logic
    let has_bringer_in_gy = state.graveyard.cards().iter()
        .any(|c| c.name() == "Bringer of the Last Gift");
    let has_spider_in_hand = state.hand.cards().iter()
        .any(|c| c.name() == "Superior Spider-Man");
    let has_bringer_in_hand = state.hand.cards().iter()
        .any(|c| c.name() == "Bringer of the Last Gift");
    let land_count = state.battlefield.permanents().iter()
        .filter(|p| matches!(p.card, Card::Land(_)))
        .count();

    let mut selected_idx: Option<usize> = None;

    // Priority 1: Spider-Man if we need it for the combo
    if has_bringer_in_gy && !has_spider_in_hand {
        for (idx, card) in milled.iter().enumerate() {
            if card.name() == "Superior Spider-Man" {
                selected_idx = Some(idx);
                if verbose {
                    println!("    Overlord returns Superior Spider-Man (combo piece!)");
                }
                break;
            }
        }
    }

    // Priority 2: Kiora if Bringer is stuck in hand
    if selected_idx.is_none() && has_bringer_in_hand {
        for (idx, card) in milled.iter().enumerate() {
            if card.name() == "Kiora, the Rising Tide" {
                selected_idx = Some(idx);
                if verbose {
                    println!("    Overlord returns Kiora (need to discard Bringer from hand)");
                }
                break;
            }
        }
    }

    // Priority 3: Town Greeter if early game
    if selected_idx.is_none() && land_count < 4 {
        for (idx, card) in milled.iter().enumerate() {
            if card.name() == "Town Greeter" {
                selected_idx = Some(idx);
                if verbose {
                    println!("    Overlord returns Town Greeter (cheap enabler)");
                }
                break;
            }
        }
    }

    // Otherwise: DON'T return anything! Leave creatures in graveyard for reanimation
    if selected_idx.is_none() && verbose {
        println!("    Overlord returns nothing (keeping creatures for reanimate)");
    }

    // Add cards to graveyard or hand
    for (idx, card) in milled.into_iter().enumerate() {
        if Some(idx) == selected_idx {
            state.hand.add_card(card);
        } else {
            state.graveyard.add_card(card);
        }
    }
}

/// Resolve Town Greeter ETB ability: mill 4, may return a land
/// Called when Spider-Man copies Town Greeter to dig for Bringer
pub fn resolve_town_greeter_etb(state: &mut GameState, verbose: bool) {
    let milled = state.library.mill(4);
    let mut milled_cards = Vec::new();
    for card in milled {
        milled_cards.push(card);
    }

    if verbose {
        let mill_names: Vec<String> = milled_cards.iter().map(|c| c.name().to_string()).collect();
        println!("    Mill 4: {}", mill_names.join(", "));
    }

    // Find the best land to return
    let mut best_land: Option<Card> = None;
    let mut best_land_idx: Option<usize> = None;

    for (idx, card) in milled_cards.iter().enumerate() {
        if matches!(card, Card::Land(_)) {
            // Prefer untapped lands, then multi-color lands
            if let Some(ref current_best) = best_land {
                let new_is_better = match (card, current_best) {
                    (Card::Land(new_land), Card::Land(current_land)) => {
                        let new_tapped = new_land.enters_tapped;
                        let current_tapped = current_land.enters_tapped;
                        if new_tapped != current_tapped {
                            !new_tapped // Prefer untapped
                        } else {
                            new_land.colors.len() > current_land.colors.len() // Prefer multi-color
                        }
                    }
                    _ => false,
                };
                if new_is_better {
                    best_land = Some(card.clone());
                    best_land_idx = Some(idx);
                }
            } else {
                best_land = Some(card.clone());
                best_land_idx = Some(idx);
            }
        }
    }

    // Return the best land to hand, rest to graveyard
    for (idx, card) in milled_cards.into_iter().enumerate() {
        if Some(idx) == best_land_idx {
            if verbose {
                println!("    -> Returned to hand: {}", card.name());
            }
            state.hand.add_card(card);
        } else {
            state.graveyard.add_card(card);
        }
    }
}

/// Resolve Kiora's ETB ability: draw 2, discard 2.
/// Discards use the shared DecisionEngine::choose_discard priority
/// (reanimation targets first, then blanks, then the last non-Spider-Man card).
pub fn resolve_kiora_etb(state: &mut GameState, verbose: bool) {
    // Draw 2, discard 2
    let hand_before = state.hand.size();
    state.draw_card();
    state.draw_card();

    // Collect drawn cards for logging
    let drawn: Vec<String> = state.hand.cards()
        .iter()
        .skip(hand_before)
        .map(|c| c.name().to_string())
        .collect();

    if verbose {
        println!("    Kiora ETB: drew {}", drawn.join(", "));
    }

    // Discard 2 - shared priority: reanimation targets first, then blanks
    let mut discarded: Vec<String> = Vec::new();
    for _ in 0..2 {
        if let Some(idx) = DecisionEngine::choose_discard(state) {
            if let Some(card) = state.hand.remove_card(idx) {
                let card_name = card.name().to_string();
                state.graveyard.add_card(card);
                discarded.push(card_name);
            }
        } else {
            break;
        }
    }

    if verbose {
        println!("    Kiora ETB: discarded {}", discarded.join(", "));
    }
}

/// Mill N cards straight into the graveyard
pub fn resolve_mill_n(state: &mut GameState, count: usize, verbose: bool) {
    let milled = state.library.mill(count);
    if verbose && !milled.is_empty() {
        let names: Vec<&str> = milled.iter().map(|c| c.name()).collect();
        println!("    Mill {}: {}", count, names.join(", "));
    }
    for card in milled {
        state.graveyard.add_card(card);
    }
}

/// Resolve Fear of Missing Out ETB: discard a card, then draw a card
pub fn resolve_fear_of_missing_out_etb(state: &mut GameState, verbose: bool) {
    if let Some(idx) = DecisionEngine::choose_discard(state) {
        if let Some(card) = state.hand.remove_card(idx) {
            if verbose {
                println!("    Fear of Missing Out: discarded {}", card.name());
            }
            state.graveyard.add_card(card);
        }
    }
    if state.draw_card() {
        if verbose {
            if let Some(card) = state.hand.cards().last() {
                println!("    Fear of Missing Out: drew {}", card.name());
            }
        }
    }
}

/// Bounce up to `max` of our own ETB-value permanents back to hand so they can
/// be recast (Does Machines, impending Overlord, fresh Fear of Missing Out)
pub fn resolve_bounce_recast_targets(state: &mut GameState, max: usize, verbose: bool) {
    let mut indices = DecisionEngine::choose_bounce_targets(state, max);
    // Remove from the highest index first so earlier indices stay valid
    indices.sort_by(|a, b| b.cmp(a));
    for idx in indices {
        if let Some(perm) = state.battlefield.remove_permanent(idx) {
            if verbose {
                println!("    Bounced {} to hand (recast it for its ETB)", perm.card.name());
            }
            state.hand.add_card(perm.card);
        }
    }
}

/// Bitter Triumph may destroy one of OUR creatures to put it into the graveyard.
/// Only do this when the combo needs a creature there: Ardyn is in the graveyard
/// with no other creature (Starscourge needs fodder). Pick a spare creature whose
/// ETB already resolved.
fn bitter_triumph_self_kill_target(state: &GameState) -> Option<usize> {
    let has_ardyn_in_gy = state.graveyard.cards().iter()
        .any(|c| c.name() == "Ardyn, the Usurper");
    let other_creatures_in_gy = state.graveyard.cards().iter()
        .filter(|c| matches!(c, Card::Creature(_)) && c.name() != "Ardyn, the Usurper")
        .count();
    if !has_ardyn_in_gy || other_creatures_in_gy > 0 {
        return None;
    }

    const SPARE_CREATURES: &[&str] = &["Dark Matter Manipulator", "Fear of Missing Out"];
    state.battlefield.permanents().iter().position(|p| {
        SPARE_CREATURES.contains(&p.card.name()) && p.is_copy_of.is_none()
    })
}

/// Bargain fodder for Beseech the Mirror, best first.
/// Enchantment creatures and artifact creatures are the best bargain because
/// the mass reanimate returns them: an impending Overlord (sacrificing it even
/// upgrades it to a full creature), then Fear of Missing Out, then Wreckage
/// Wickerfolk, then a flipped Overlord. Next a Dragonstorm on the battlefield
/// (its ETB is already used; a Dragon entering would return it, but that is
/// smaller value than the free-cast). Surveil and
/// bargain are an either/or choice for each Grim Bauble (the surveil sacrifices
/// it): bargain a Bauble before Does Machines only when no mana is free for its
/// surveil this turn. When the surveil is affordable, keep the Bauble for it,
/// bargain Does Machines instead, and use the Bauble only as a last resort.
pub fn beseech_bargain_fodder(state: &GameState) -> Option<usize> {
    let perms = state.battlefield.permanents();
    if let Some(i) = perms.iter().position(|p| {
        p.card.name() == "Overlord of the Balemurk"
            && p.get_counter(CounterType::Time) > 0
            && p.is_copy_of.is_none()
    }) {
        return Some(i);
    }
    if let Some(i) = perms.iter().position(|p| {
        p.card.name() == "Fear of Missing Out" && p.is_copy_of.is_none()
    }) {
        return Some(i);
    }
    if let Some(i) = perms.iter().position(|p| {
        p.card.name() == "Wreckage Wickerfolk" && p.is_copy_of.is_none()
    }) {
        return Some(i);
    }
    if let Some(i) = perms.iter().position(|p| {
        p.card.name() == "Overlord of the Balemurk" && p.is_copy_of.is_none()
    }) {
        return Some(i);
    }
    if let Some(i) = perms.iter().position(|p| {
        matches!(p.card.name(), "Roiling Dragonstorm" | "Corroding Dragonstorm")
            && p.is_copy_of.is_none()
    }) {
        return Some(i);
    }
    let bauble_idx = perms.iter().position(|p| {
        p.card.name() == "Grim Bauble" && p.is_copy_of.is_none()
    });
    if bauble_idx.is_some() {
        let surveil_cost = ManaCost { black: 1, generic: 2, ..Default::default() };
        let surveil_affordable = crate::game::mana::can_afford_cost(&surveil_cost, state, None);
        if !surveil_affordable {
            return bauble_idx;
        }
    }
    perms.iter().position(|p| p.card.name() == "Does Machines" && p.is_copy_of.is_none())
        .or(bauble_idx)
}

/// Decide what Beseech the Mirror would fetch right now.
/// Returns (card name to fetch, try to free-cast it with bargain).
fn beseech_plan(state: &GameState) -> Option<(&'static str, bool)> {
    let in_hand = |name: &str| state.hand.cards().iter().any(|c| c.name() == name);
    let in_library = |name: &str| state.library.cards().iter().any(|c| c.name() == name);

    let has_bringer_in_gy = state.graveyard.cards().iter()
        .any(|c| c.name() == "Bringer of the Last Gift");
    let has_ardyn_in_gy = state.graveyard.cards().iter()
        .any(|c| c.name() == "Ardyn, the Usurper");
    let other_creatures_in_gy = state.graveyard.cards().iter()
        .filter(|c| matches!(c, Card::Creature(_)) && c.name() != "Ardyn, the Usurper")
        .count();
    let combo_target_in_gy = has_bringer_in_gy || (has_ardyn_in_gy && other_creatures_in_gy >= 1);

    if combo_target_in_gy {
        // Get Superior Spider-Man; free-cast it only when the combo wins within two turns
        if !in_hand("Superior Spider-Man") && in_library("Superior Spider-Man") {
            let lethal = has_bringer_in_gy && is_combo_lethal_within_two_turns(state);
            let can_free_cast = (lethal || (has_ardyn_in_gy && !has_bringer_in_gy))
                && beseech_bargain_fodder(state).is_some();
            return Some(("Superior Spider-Man", can_free_cast));
        }
        return None;
    }

    // No target in the graveyard yet. With bargain fodder, free-cast Marang's
    // omen side (Coil and Catch, MV 4): draw 3, discard 1. It digs and bins a
    // target in one card, so it beats a free Bitter Triumph.
    if beseech_bargain_fodder(state).is_some() && in_library("Marang River Regent") {
        return Some(("Marang River Regent", true));
    }

    // Otherwise get a discard outlet or Bringer
    let target_in_hand = DecisionEngine::reanimation_target_in_hand(state).is_some();
    if target_in_hand && !in_hand("Bitter Triumph") && in_library("Bitter Triumph") {
        // Bitter Triumph is MV 2, so a bargained Beseech casts it free
        let can_free_cast = beseech_bargain_fodder(state).is_some();
        return Some(("Bitter Triumph", can_free_cast));
    }
    if !in_hand("Bringer of the Last Gift") && in_library("Bringer of the Last Gift") {
        // MV 8: always goes to hand, so never bargain for it
        return Some(("Bringer of the Last Gift", false));
    }

    None
}

/// Check whether casting Beseech the Mirror right now does something useful
pub fn beseech_useful(state: &GameState) -> bool {
    beseech_plan(state).is_some()
}

/// Resolve Coil and Catch (Marang River Regent's omen side): draw 3, discard 1,
/// then shuffle the card into the library.
pub fn resolve_coil_and_catch(
    state: &mut GameState,
    marang: Card,
    rng: &mut crate::rng::GameRng,
    verbose: bool,
) {
    for _ in 0..3 {
        state.draw_card();
    }
    if let Some(didx) = DecisionEngine::choose_discard(state) {
        if let Some(discarded) = state.hand.remove_card(didx) {
            if verbose {
                println!("    Discarded: {}", discarded.name());
            }
            state.graveyard.add_card(discarded);
        }
    }
    state.library.add_card(marang);
    state.library.shuffle(rng);
}

/// Resolve Beseech the Mirror: optionally bargain (sacrifice fodder), search the
/// library for the planned card, and cast it free (MV 4 or less, bargained) or
/// put it into hand.
pub fn resolve_beseech(
    state: &mut GameState,
    db: &CardDatabase,
    rng: &mut crate::rng::GameRng,
    verbose: bool,
) {
    let Some((fetch_name, mut free_cast)) = beseech_plan(state) else {
        if verbose {
            println!("    Beseech the Mirror: no useful fetch");
        }
        return;
    };

    if free_cast {
        if let Some(idx) = beseech_bargain_fodder(state) {
            if let Some(perm) = state.battlefield.remove_permanent(idx) {
                if verbose {
                    println!("    Beseech the Mirror: bargained {} (sacrificed)", perm.card.name());
                }
                state.graveyard.add_card(perm.card);
            }
        } else {
            free_cast = false;
        }
    }

    let Some(lib_idx) = state.library.cards().iter().position(|c| c.name() == fetch_name) else {
        if verbose {
            println!("    Beseech the Mirror: {} not found in library", fetch_name);
        }
        return;
    };
    let card = state.library.cards_mut().remove(lib_idx);
    state.library.shuffle(rng);

    if free_cast && card.name() == "Marang River Regent" {
        // Cast the omen side (Coil and Catch, MV 4) for free
        if verbose {
            println!("    Beseech the Mirror: casts Coil and Catch (Marang's omen) for free!");
        }
        resolve_coil_and_catch(state, card, rng, verbose);
    } else if free_cast && card.mana_value() <= 4 {
        if verbose {
            println!("    Beseech the Mirror: casts {} for free!", card.name());
        }
        match &card {
            Card::Creature(_) => {
                let _ = cast_creature(state, &card, false);

                // Process ETB triggers (same dance as the engine)
                let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                if perm_idx < state.battlefield.permanents().len() {
                    let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                    let perm_name = perm.card.name().to_string();
                    let perm_turn = perm.turn_entered;
                    let _ = process_etb_triggers_verbose(state, &mut perm, db, verbose, rng);

                    if let Some(idx) = state.battlefield.permanents().iter().position(|p| {
                        p.card.name() == perm_name && p.turn_entered == perm_turn
                    }) {
                        state.battlefield.permanents_mut()[idx] = perm;
                    }
                }
            }
            Card::Land(_) => {
                // Should not happen (we never fetch lands); keep it in hand
                state.hand.add_card(card);
            }
            _ => {
                let _ = cast_spell(state, &card, db, verbose, rng);
            }
        }
    } else {
        if verbose {
            println!("    Beseech the Mirror: put {} into hand", card.name());
        }
        state.hand.add_card(card);
    }
}

/// Deceit's evoke cost is {U/B}{U/B}. Pick how to pay it: prefer {U}{U} when a
/// bounce target exists (the blue ETB mode returns one of our own permanents to
/// hand), otherwise any affordable mix. Returns (cost, bounce).
pub fn deceit_evoke_payment(state: &GameState) -> (ManaCost, bool) {
    let uu = ManaCost { blue: 2, ..Default::default() };
    let has_bounce_target = !DecisionEngine::choose_bounce_targets(state, 1).is_empty();
    if has_bounce_target && crate::game::mana::can_afford_cost(&uu, state, None) {
        return (uu, true);
    }

    let bb = ManaCost { black: 2, ..Default::default() };
    let ub = ManaCost { blue: 1, black: 1, ..Default::default() };
    for cost in [bb.clone(), ub, uu] {
        if crate::game::mana::can_afford_cost(&cost, state, None) {
            return (cost, false);
        }
    }
    (bb, false)
}

/// Check whether Deceit can be evoked right now ({U/B}{U/B} in any mix)
pub fn can_afford_deceit_evoke(state: &GameState) -> bool {
    let costs = [
        ManaCost { black: 2, ..Default::default() },
        ManaCost { blue: 1, black: 1, ..Default::default() },
        ManaCost { blue: 2, ..Default::default() },
    ];
    costs.iter().any(|c| crate::game::mana::can_afford_cost(c, state, None))
}

/// Resolve Formidable Speaker's ETB ability
///
/// May discard a card to search library for a creature card and put it into hand.
/// Decision logic:
/// - Only use if we have something good to discard (Bringer/Terror) AND need Spider-Man
/// - Or discard a land to find a combo piece
pub fn resolve_formidable_speaker_etb(state: &mut GameState, rng: &mut crate::rng::GameRng, verbose: bool) {
    // Check if we want to use the ability
    // We want to discard if:
    // 1. We have Bringer or Terror in hand (want them in graveyard) AND don't have Spider-Man
    // 2. We have Spider-Man but no Bringer in graveyard (can discard Bringer to tutor Spider-Man)

    let has_spider_man = state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man");
    let has_bringer_in_hand = state.hand.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
    let has_terror_in_hand = state.hand.cards().iter().any(|c| c.name() == "Terror of the Peaks");
    let has_ardyn_in_hand = state.hand.cards().iter().any(|c| c.name() == "Ardyn, the Usurper");
    let has_bringer_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
    let has_terror_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Terror of the Peaks");

    // Determine what to discard and what to tutor
    let mut discard_target: Option<String> = None;
    let mut tutor_target: Option<String> = None;

    // Priority 1: Discard Bringer/Terror/Ardyn to get Spider-Man
    // Only if Spider-Man is actually in the library
    let spider_in_library = state.library.cards().iter()
        .any(|c| c.name() == "Superior Spider-Man");

    if !has_spider_man && spider_in_library {
        if has_bringer_in_hand {
            discard_target = Some("Bringer of the Last Gift".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        } else if has_terror_in_hand {
            discard_target = Some("Terror of the Peaks".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        } else if has_ardyn_in_hand {
            discard_target = Some("Ardyn, the Usurper".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        }
    }

    // Priority 1.5: If Spider-Man is NOT in hand, Bringer IS in graveyard (combo is ready!),
    // discard ANY non-essential card to tutor for Spider-Man.
    // This handles late-game scenarios where we've already discarded Bringer/Terror via milling
    // but have mill creatures in hand that are less valuable than completing the combo.
    if tutor_target.is_none() && !has_spider_man && has_bringer_in_gy && spider_in_library {
        // Count mill creatures in hand to find excess
        let kiora_count = state.hand.cards().iter()
            .filter(|c| c.name() == "Kiora, the Rising Tide")
            .count();
        let town_greeter_count = state.hand.cards().iter()
            .filter(|c| c.name() == "Abuelo's Awakening" || c.name() == "Town Greeter")
            .count();
        let has_overlord_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Overlord of the Balemurk");

        // Priority for discarding: excess Kiora > excess Town Greeter > any mill creature
        if kiora_count > 1 {
            discard_target = Some("Kiora, the Rising Tide".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        } else if town_greeter_count > 1 {
            discard_target = Some("Town Greeter".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        } else if kiora_count >= 1 {
            // We have exactly 1 Kiora - discard it because combo is more important
            discard_target = Some("Kiora, the Rising Tide".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        } else if town_greeter_count >= 1 {
            discard_target = Some("Town Greeter".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        } else if has_overlord_in_hand {
            // Overlord is also a valid discard target if that's all we have
            discard_target = Some("Overlord of the Balemurk".to_string());
            tutor_target = Some("Superior Spider-Man".to_string());
        }
    }

    // Priority 2: If we have Spider-Man but no Bringer in graveyard, discard Bringer
    if tutor_target.is_none() && has_spider_man && !has_bringer_in_gy && has_bringer_in_hand {
        discard_target = Some("Bringer of the Last Gift".to_string());
        // Tutor for Terror if we don't have it in graveyard, otherwise tutor for mill creature
        if !has_terror_in_gy && !has_terror_in_hand {
            tutor_target = Some("Terror of the Peaks".to_string());
        } else {
            // Terror is already in graveyard, tutor for mill creature to add damage
            // Priority: Overlord > Kiora > second Spider-Man
            let has_overlord_in_hand = state.hand.cards().iter().any(|c| c.name() == "Overlord of the Balemurk");
            let has_kiora_in_hand = state.hand.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide");
            
            if !has_overlord_in_hand {
                tutor_target = Some("Overlord of the Balemurk".to_string());
            } else if !has_kiora_in_hand {
                tutor_target = Some("Kiora, the Rising Tide".to_string());
            } else {
                // Already have mill creatures, tutor for backup Spider-Man if < 2 in hand
                let spider_count = state.hand.cards().iter().filter(|c| c.name() == "Superior Spider-Man").count();
                if spider_count < 2 {
                    tutor_target = Some("Superior Spider-Man".to_string());
                }
            }
        }
    }


    // Priority 3: If we have Spider-Man and Bringer in graveyard, but no Terror
    if tutor_target.is_none() && has_spider_man && has_bringer_in_gy && !has_terror_in_gy && !has_terror_in_hand {
        // Find something to discard (prefer lands or duplicates)
        let land_idx = state.hand.cards().iter()
            .position(|c| matches!(c, Card::Land(_)));
        if land_idx.is_some() {
            // Just find any discard target, we want Terror
            discard_target = Some("land".to_string());
            tutor_target = Some("Terror of the Peaks".to_string());
        }
    }

    // Priority 4: If we have Spider-Man, and Terror in GY but no Bringer in GY or hand
    // We need to get Bringer somehow - BUT only if Bringer is in the library!
    // Also skip if we have the Ardyn combo available
    if tutor_target.is_none() && has_spider_man && has_terror_in_gy && !has_bringer_in_gy && !has_bringer_in_hand {
        // Check if Ardyn combo is available (skip Priority 4 if so - Ardyn is a valid path)
        let has_ardyn_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Ardyn, the Usurper");
        let other_creatures_count = state.graveyard.cards().iter()
            .filter(|c| matches!(c, Card::Creature(_)) && c.name() != "Ardyn, the Usurper")
            .count();

        // Only try to tutor Bringer if we don't have Ardyn combo available
        if !(has_ardyn_in_gy && other_creatures_count >= 1) {
            // Check if Bringer is actually in the library
            let bringer_in_library = state.library.cards().iter()
                .any(|c| c.name() == "Bringer of the Last Gift");

            if bringer_in_library {
                // Find something to discard (prefer excess lands)
                let lands_in_hand: Vec<usize> = state.hand.cards().iter()
                    .enumerate()
                    .filter(|(_, c)| matches!(c, Card::Land(_)))
                    .map(|(i, _)| i)
                    .collect();

                // Only discard if we have 2+ lands (keep at least 1 for land drop)
                if lands_in_hand.len() >= 2 {
                    discard_target = Some("land".to_string());
                    tutor_target = Some("Bringer of the Last Gift".to_string());
                }
            }
        }
    }

    // Priority 5: If we have Spider-Man and Bringer in GY but need more creatures for damage
    // Tutor for mill creatures (Overlord/Kiora) to add to graveyard value
    if tutor_target.is_none() && has_spider_man && has_bringer_in_gy {
        let has_overlord = state.hand.cards().iter().any(|c| c.name() == "Overlord of the Balemurk")
            || state.graveyard.cards().iter().any(|c| c.name() == "Overlord of the Balemurk");
        let has_kiora = state.hand.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide")
            || state.graveyard.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide");

        // Find something to discard (prefer excess lands)
        let lands_in_hand: Vec<usize> = state.hand.cards().iter()
            .enumerate()
            .filter(|(_, c)| matches!(c, Card::Land(_)))
            .map(|(i, _)| i)
            .collect();

        if lands_in_hand.len() >= 2 {
            if !has_overlord {
                discard_target = Some("land".to_string());
                tutor_target = Some("Overlord of the Balemurk".to_string());
            } else if !has_kiora {
                discard_target = Some("land".to_string());
                tutor_target = Some("Kiora, the Rising Tide".to_string());
            }
        }
    }

    // Priority 6: If we have Spider-Man and Ardyn in graveyard (but no Bringer),
    // and there are creatures for Starscourge - this is a valid combo!
    // Tutor for Terror if we need it, otherwise get more creatures
    if tutor_target.is_none() && has_spider_man && !has_bringer_in_gy {
        let has_ardyn_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Ardyn, the Usurper");
        let other_creatures_count = state.graveyard.cards().iter()
            .filter(|c| matches!(c, Card::Creature(_)) && c.name() != "Ardyn, the Usurper")
            .count();

        // Valid Ardyn combo: Ardyn + at least 1 other creature for Starscourge
        if has_ardyn_in_gy && other_creatures_count >= 1 {
            // Find something to discard (prefer excess lands)
            let lands_in_hand: Vec<usize> = state.hand.cards().iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Card::Land(_)))
                .map(|(i, _)| i)
                .collect();

            if lands_in_hand.len() >= 2 {
                // If no Terror in GY, tutor for it
                if !has_terror_in_gy && !has_terror_in_hand {
                    discard_target = Some("land".to_string());
                    tutor_target = Some("Terror of the Peaks".to_string());
                } else {
                    // Already have Terror, tutor for more creatures to add damage
                    let has_overlord = state.hand.cards().iter().any(|c| c.name() == "Overlord of the Balemurk")
                        || state.graveyard.cards().iter().any(|c| c.name() == "Overlord of the Balemurk");
                    let has_kiora = state.hand.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide")
                        || state.graveyard.cards().iter().any(|c| c.name() == "Kiora, the Rising Tide");
                    let spider_count = state.hand.cards().iter()
                        .filter(|c| c.name() == "Superior Spider-Man")
                        .count();

                    if !has_overlord {
                        discard_target = Some("land".to_string());
                        tutor_target = Some("Overlord of the Balemurk".to_string());
                    } else if !has_kiora {
                        discard_target = Some("land".to_string());
                        tutor_target = Some("Kiora, the Rising Tide".to_string());
                    } else if spider_count < 2 {
                        // Backup Spider-Man for redundancy
                        discard_target = Some("land".to_string());
                        tutor_target = Some("Superior Spider-Man".to_string());
                    }
                    // If we have everything, don't waste the ability
                }
            }
        }
    }

    // Execute the ability if we have targets
    if let (Some(discard), Some(tutor)) = (&discard_target, &tutor_target) {
        // Find and discard the card
        let discard_idx = if discard == "land" {
            state.hand.cards().iter()
                .position(|c| matches!(c, Card::Land(_)))
        } else {
            state.hand.cards().iter()
                .position(|c| c.name() == discard)
        };

        if let Some(idx) = discard_idx {
            if let Some(card) = state.hand.remove_card(idx) {
                let discarded_name = card.name().to_string();
                state.graveyard.add_card(card);

                // Search library for the tutor target
                let tutor_idx = state.library.cards().iter()
                    .position(|c| c.name() == tutor);

                if let Some(lib_idx) = tutor_idx {
                    // Remove card from library using cards_mut
                    let tutored = state.library.cards_mut().remove(lib_idx);
                    let tutored_name = tutored.name().to_string();
                    state.hand.add_card(tutored);
                    state.library.shuffle(rng);

                    if verbose {
                        println!("    Formidable Speaker ETB: discarded {}, tutored {}",
                            discarded_name, tutored_name);
                    }
                } else if verbose {
                    println!("    Formidable Speaker ETB: discarded {}, but {} not found in library",
                        discarded_name, tutor);
                }
            }
        }
    } else if verbose {
        println!("    Formidable Speaker ETB: chose not to discard");
    }
}

/// Check if Ardyn, the Usurper is on the battlefield
fn has_ardyn_on_battlefield(state: &GameState) -> bool {
    state.battlefield.permanents().iter().any(|p| {
        p.card.name() == "Ardyn, the Usurper"
            || p.is_copy_of.as_deref() == Some("Ardyn, the Usurper")
    })
}

/// Check if a creature card is a Demon
fn is_creature_demon(card: &Card) -> bool {
    match card {
        Card::Creature(c) => c.creature_types.iter().any(|t| t == "Demon"),
        _ => false,
    }
}

/// Calculate total damage from the combo if cast now
///
/// Damage sources:
/// 1. Terror triggers from creatures entering (both from battlefield and graveyard)
/// 2. Combat damage from creatures already on battlefield (no summoning sickness)
/// 3. Combat damage from Demons with haste (if Ardyn is on battlefield)
pub fn calculate_combo_damage(state: &GameState) -> u32 {
    // Check if Ardyn is on battlefield (Demons get haste)
    let ardyn_on_battlefield = has_ardyn_on_battlefield(state);

    // Creatures that would be reanimated from graveyard
    let creatures_in_graveyard: Vec<&Card> = state
        .graveyard
        .cards()
        .iter()
        .filter(|c| matches!(c, Card::Creature(_)))
        .collect();

    // User rule: anything Superior Spider-Man copies is a 4/4. The copy enters
    // as Bringer in name and types, but with Spider-Man's own 4 power.
    const SPIDER_COPY_POWER: u32 = 4;

    // Count Terrors that will be on battlefield after combo
    let terrors_in_graveyard = creatures_in_graveyard
        .iter()
        .filter(|c| c.name() == "Terror of the Peaks")
        .count() as u32;

    let terrors_on_battlefield = state
        .battlefield
        .permanents()
        .iter()
        .filter(|p| {
            p.card.name() == "Terror of the Peaks" || p.is_copy_of.as_deref() == Some("Terror of the Peaks")
        })
        .count() as u32;

    // Calculate Terror trigger damage (IMMEDIATE)
    // When Spider-Man enters as a copy of Bringer, creatures are reanimated
    // Each Terror triggers for each creature entering (except itself)
    //
    // IMPORTANT: Spider-Man entering does NOT trigger Terrors because Terror is
    // still in the graveyard at that point. Terrors only trigger for the creatures
    // that enter simultaneously with them during the mass reanimate.

    let mut terror_damage = 0u32;

    // Terrors already on battlefield trigger for EACH creature entering (including Spider-Man)
    if terrors_on_battlefield > 0 {
        terror_damage += SPIDER_COPY_POWER * terrors_on_battlefield;
        for creature in &creatures_in_graveyard {
            if let Card::Creature(c) = creature {
                terror_damage += c.power * terrors_on_battlefield;
            }
        }
    }

    // Terrors from graveyard trigger for creatures entering AT THE SAME TIME (during mass reanimate)
    // They DON'T trigger for Spider-Man (Spider-Man entered BEFORE the mass reanimate)
    // They trigger for all other creatures entering simultaneously, but NOT for themselves
    if terrors_in_graveyard > 0 {
        // Each creature from graveyard triggers Terrors from graveyard (except Terror doesn't trigger for itself)
        for creature in &creatures_in_graveyard {
            if let Card::Creature(c) = creature {
                if c.base.name == "Terror of the Peaks" {
                    // A Terror entering triggers all OTHER Terrors (from graveyard only - battlefield Terrors already triggered above)
                    terror_damage += c.power * (terrors_in_graveyard - 1);
                } else {
                    terror_damage += c.power * terrors_in_graveyard;
                }
            }
        }
    }

    // Combat damage from creatures that can attack THIS turn (already on battlefield, no summoning sickness)
    // These creatures will attack after we cast the combo in main phase 1
    // Exception: Demons have haste if Ardyn is on battlefield
    let current_combat_power: u32 = state
        .battlefield
        .permanents()
        .iter()
        .filter(|p| {
            if !matches!(p.card, Card::Creature(_)) {
                return false;
            }
            // Check for impending counters
            if let Some(counter_count) = p.counters.get(&crate::game::zones::CounterType::Time) {
                if *counter_count > 0 {
                    return false;
                }
            }
            // Check summoning sickness
            let has_summoning_sickness = state.turn <= p.turn_entered;
            if has_summoning_sickness {
                // Demons get haste from Ardyn
                if ardyn_on_battlefield && is_creature_demon(&p.card) {
                    return true; // Can attack despite summoning sickness
                }
                return false;
            }
            true
        })
        .map(|p| {
            if let Card::Creature(c) = &p.card {
                c.power
            } else {
                0
            }
        })
        .sum();

    // Calculate combat damage from reanimated Demons (if Ardyn gives them haste)
    let reanimated_demon_combat_power: u32 = if ardyn_on_battlefield {
        // Bringer of the Last Gift is a Demon and would be reanimated
        // Spider-Man (as a copy of Bringer) is NOT a Demon
        // Any Demons in graveyard would get haste from Ardyn after reanimate
        creatures_in_graveyard
            .iter()
            .filter(|c| is_creature_demon(c))
            .map(|c| {
                if let Card::Creature(creature) = c {
                    creature.power
                } else {
                    0
                }
            })
            .sum()
    } else {
        0
    };

    terror_damage + current_combat_power + reanimated_demon_combat_power
}

/// Estimate the combat damage of ONE attack step on a turn after the combo.
///
/// Models the battlefield after Spider-Man copies Bringer and the mass
/// reanimate resolves, then sums the power of each creature (goldfish rules:
/// no blockers, no removal; everything untaps and attacks):
/// - The Spider-Man that casts the combo. User rule: anything Spider-Man
///   copies is a 4/4 (the combat model also keeps copies at their own power,
///   see `simulate_combat`), so it attacks for 4, not for Bringer's 6.
/// - Every creature in the graveyard except one Bringer (Mind Swap exiles the
///   copied card); the rest are mass reanimated.
/// - Every non-impending creature already on the battlefield, including
///   Starscourge tokens. Bringer's ETB sacrifices them and the mass reanimate
///   returns them in the same resolution.
/// - Impending creatures survive the sacrifice. They attack only when their
///   time counters run out: one counter is removed in each end phase, so a
///   permanent with more than one counter cannot attack on the next turn.
/// - Dark Matter Manipulator gets +2 power for each 7 cards in the graveyard.
///   The estimate uses the cards that stay in the graveyard after the
///   reanimate (non-creature cards). Mills during the resolution only add
///   more, so this is a lower bound.
pub fn calculate_next_turn_damage(state: &GameState) -> u32 {
    const SPIDER_MAN_POWER: u32 = 4;

    // Graveyard after the combo: creatures leave (reanimated or exiled),
    // non-creature cards stay.
    let noncreature_gy = state
        .graveyard
        .cards()
        .iter()
        .filter(|c| !matches!(c, Card::Creature(_)))
        .count() as u32;
    let dmm_bonus = 2 * (noncreature_gy / 7);

    // The Spider-Man that copies Bringer
    let mut total = SPIDER_MAN_POWER;

    // Creatures mass reanimated from the graveyard (all but the exiled Bringer)
    let mut copied_bringer_skipped = false;
    for card in state.graveyard.cards() {
        if let Card::Creature(c) = card {
            if !copied_bringer_skipped && c.base.name == "Bringer of the Last Gift" {
                copied_bringer_skipped = true; // Mind Swap exiles the copied card
                continue;
            }
            total += c.power;
            if c.base.name == "Dark Matter Manipulator" {
                total += dmm_bonus;
            }
        }
    }

    // Creatures already on the battlefield: non-impending ones are sacrificed
    // and reanimated as fresh non-copies; impending ones survive as they are.
    for perm in state.battlefield.permanents() {
        if let Card::Creature(c) = &perm.card {
            let time_counters = perm.get_counter(CounterType::Time);
            if time_counters > 1 {
                continue; // still impending during the next turn's combat
            }
            total += c.power;
            if time_counters == 0 && c.base.name == "Dark Matter Manipulator" {
                total += dmm_bonus;
            }
        }
    }

    total
}

/// Estimate the total damage when the combo is cast NOW and the game runs for
/// two more turns: the immediate damage (Terror triggers plus this turn's
/// attack), plus the post-combo board attacking on each of the two following
/// turns.
pub fn calculate_two_turn_combo_damage(state: &GameState) -> u32 {
    calculate_combo_damage(state) + 2 * calculate_next_turn_damage(state)
}

/// Check if casting the combo NOW wins within two turns (goldfish rules: no
/// blockers and no removal). A win this turn also passes, so the engine never
/// waits when an immediate kill is available: the same action (cast the
/// combo) is taken, and the engine always attacks for maximum damage, so the
/// sooner win is taken automatically.
pub fn is_combo_lethal_within_two_turns(state: &GameState) -> bool {
    calculate_two_turn_combo_damage(state) >= state.opponent_life as u32
}


/// Resolve a saga chapter ability
pub fn resolve_saga_chapter(state: &mut GameState, saga_name: &str, chapter: u32, verbose: bool) {
    if saga_name == "Awaken the Honored Dead" {
        match chapter {
            1 => {
                // Chapter I: Destroy target permanent (skip for goldfishing)
                if verbose {
                    println!("    Awaken Chapter I: Destroy target permanent (skipped - no opponent)");
                }
            }
            2 => {
                // Chapter II: Mill 3
                if verbose {
                    println!("    Awaken Chapter II: Mill 3");
                }
                let mut milled = Vec::new();
                for _ in 0..3 {
                    if let Some(card) = state.library.cards_mut().pop() {
                        if verbose {
                            println!("      -> Milled: {}", card.name());
                        }
                        milled.push(card);
                    }
                }
                for card in milled {
                    state.graveyard.add_card(card);
                }
            }
            3 => {
                // Chapter III: Return creature from graveyard OR search for creature/land
                if verbose {
                    println!("    Awaken Chapter III: Return creature or search");
                }
                
                // Check if there's a creature in graveyard to return
                let creature_in_gy = state.graveyard.cards().iter()
                    .position(|c| matches!(c, Card::Creature(_)));
                
                if let Some(idx) = creature_in_gy {
                    // Return creature to hand
                    if let Some(creature) = state.graveyard.remove_card(idx) {
                        if verbose {
                            println!("      -> Returned {} from graveyard to hand", creature.name());
                        }
                        state.hand.add_card(creature);
                    }
                } else {
                    // Search library for creature or land
                    if verbose {
                        println!("      -> No creature in graveyard, searching library");
                    }
                    
                    // Priority: Spider-Man > Kiora > Formidable > Land
                    let mut target_idx = None;
                    
                    // Look for Spider-Man first
                    if target_idx.is_none() {
                        target_idx = state.library.cards().iter()
                            .position(|c| c.name() == "Superior Spider-Man");
                    }
                    
                    // Then Kiora
                    if target_idx.is_none() {
                        target_idx = state.library.cards().iter()
                            .position(|c| c.name() == "Kiora, the Rising Tide");
                    }
                    
                    // Then Formidable Speaker
                    if target_idx.is_none() {
                        target_idx = state.library.cards().iter()
                            .position(|c| c.name() == "Formidable Speaker");
                    }
                    
                    // Finally any land
                    if target_idx.is_none() {
                        target_idx = state.library.cards().iter()
                            .position(|c| matches!(c, Card::Land(_)));
                    }
                    
                    if let Some(idx) = target_idx {
                        let card = state.library.cards_mut().remove(idx);
                        if verbose {
                            println!("      -> Found and added to hand: {}", card.name());
                        }
                        state.hand.add_card(card);
                        
                        // Shuffle library (no RNG needed for goldfishing)
                        // In a real game, would shuffle here
                    }
                }
            }
            _ => {
                if verbose {
                    println!("    Unknown chapter {} for {}", chapter, saga_name);
                }
            }
        }
    }
}

#[cfg(test)]
mod combo_damage_tests {
    use super::*;
    use crate::card::types::BaseCard;
    use crate::card::types::SpellCard;
    use crate::card::{CreatureCard, ManaCost};
    use crate::game::zones::Permanent;

    #[test]
    fn test_calculate_combo_damage_no_creatures() {
        let state = GameState::new();
        let damage = calculate_combo_damage(&state);
        assert_eq!(damage, 0);
    }

    #[test]
    fn test_calculate_combo_damage_with_terror_on_battlefield() {
        let mut state = GameState::new();
        state.opponent_life = 20;

        // Add Terror to battlefield
        let terror = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Terror of the Peaks".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 4,
            },
            power: 3,
            toughness: 3,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let permanent = Permanent::new(terror, 1);
        state.battlefield.add_permanent(permanent);

        // Add Bringer to graveyard
        let bringer = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 6,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        state.graveyard.add_card(bringer);

        let damage = calculate_combo_damage(&state);
        // Terror on battlefield triggers for the Spider-Man copy (4 power,
        // user rule) + Bringer (6 power) = 10 damage
        assert_eq!(damage, 10);
    }

    #[test]
    fn test_calculate_combo_damage_with_terror_in_graveyard() {
        let mut state = GameState::new();
        state.opponent_life = 20;

        // Add Terror to graveyard
        let terror = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Terror of the Peaks".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 4,
            },
            power: 3,
            toughness: 3,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        state.graveyard.add_card(terror);

        // Add Bringer to graveyard
        let bringer = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 6,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        state.graveyard.add_card(bringer);

        let damage = calculate_combo_damage(&state);
        // Terrors from graveyard trigger for creatures entering AT THE SAME TIME
        // Bringer (6 power) triggers Terror from graveyard = 6 damage
        // Terror (3 power) triggers OTHER Terrors from graveyard (terrorsInGraveyard - 1 = 0) = 0 damage
        // Total: 6 damage
        assert_eq!(damage, 6);
    }

    #[test]
    fn test_calculate_combo_damage_with_combat_creatures() {
        let mut state = GameState::new();
        state.turn = 3; // Avoid summoning sickness

        // Add a creature to battlefield that can attack
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 2,
            },
            power: 4,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let permanent = Permanent::new(creature, 1); // Entered on turn 1, now turn 3
        state.battlefield.add_permanent(permanent);

        let damage = calculate_combo_damage(&state);
        // Combat damage from creature with no summoning sickness
        assert_eq!(damage, 4);
    }

    #[test]
    fn test_calculate_combo_damage_summoning_sickness() {
        let mut state = GameState::new();
        state.turn = 2;

        // Add a creature to battlefield that just entered (summoning sickness)
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 2,
            },
            power: 4,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let permanent = Permanent::new(creature, 2); // Entered on turn 2, now turn 2
        state.battlefield.add_permanent(permanent);

        let damage = calculate_combo_damage(&state);
        // No combat damage due to summoning sickness
        assert_eq!(damage, 0);
    }

    /// Build a bare creature card for tests
    fn test_creature(name: &str, power: u32) -> Card {
        Card::Creature(CreatureCard {
            base: BaseCard {
                name: name.to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 0,
            },
            power,
            toughness: power,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        })
    }

    #[test]
    fn test_terror_trigger_counts_spider_copy_as_4_power() {
        // User rule: anything Spider-Man copies is a 4/4. A Terror on the
        // battlefield triggers for the entering Spider-Man copy with 4 power,
        // not with Bringer's 6.
        let mut state = GameState::new();
        let terror = Permanent::new(test_creature("Terror of the Peaks", 5), 1);
        state.battlefield.add_permanent(terror);
        state.graveyard.add_card(test_creature("Bringer of the Last Gift", 6));

        // Spider-Man copy 4 + reanimated Bringer 6, one Terror = 10
        assert_eq!(calculate_combo_damage(&state), 10);
    }

    #[test]
    fn test_two_turn_gate_yes_when_one_turn_gate_says_no() {
        // Spider-Man in hand, Bringer and Valgavoth in the graveyard, no
        // Terror, opponent at 20. No damage this turn, but the post-combo
        // board (Spider-Man 4 + Valgavoth 9 = 13 per attack) wins within two
        // turns. The old one-turn gate said no; the new gate says yes.
        let mut state = GameState::new();
        state.opponent_life = 20;
        state.hand.add_card(test_creature("Superior Spider-Man", 4));
        state.graveyard.add_card(test_creature("Bringer of the Last Gift", 6));
        state.graveyard.add_card(test_creature("Valgavoth, Terror Eater", 9));

        // Old one-turn answer: no Terror, nothing attacks this turn -> 0 damage
        assert_eq!(calculate_combo_damage(&state), 0);
        assert!(calculate_combo_damage(&state) < state.opponent_life as u32);

        // The copied Bringer is exiled; Valgavoth and Spider-Man attack
        assert_eq!(calculate_next_turn_damage(&state), 13);
        assert!(is_combo_lethal_within_two_turns(&state));
    }

    #[test]
    fn test_two_turn_gate_no_when_board_too_small() {
        // Only Bringer in the graveyard: after the combo the board is one
        // Spider-Man copy (4 power). 8 damage in two turns < 20 life.
        let mut state = GameState::new();
        state.opponent_life = 20;
        state.graveyard.add_card(test_creature("Bringer of the Last Gift", 6));

        assert_eq!(calculate_next_turn_damage(&state), 4);
        assert!(!is_combo_lethal_within_two_turns(&state));
    }

    #[test]
    fn test_next_turn_damage_impending_and_dmm_bonus() {
        let mut state = GameState::new();

        // Impending Overlord with 4 time counters: cannot attack next turn
        let mut overlord_far = Permanent::new(test_creature("Overlord of the Balemurk", 5), 1);
        overlord_far.add_counter(CounterType::Time, 4);
        state.battlefield.add_permanent(overlord_far);

        // Impending Overlord with 1 time counter: the counter is removed in
        // this turn's end phase, so it attacks next turn
        let mut overlord_near = Permanent::new(test_creature("Overlord of the Balemurk", 5), 1);
        overlord_near.add_counter(CounterType::Time, 1);
        state.battlefield.add_permanent(overlord_near);

        // Graveyard: the copied Bringer (exiled, not counted), a Dark Matter
        // Manipulator, and 14 non-creature cards (stay after the reanimate)
        state.graveyard.add_card(test_creature("Bringer of the Last Gift", 6));
        state.graveyard.add_card(test_creature("Dark Matter Manipulator", 1));
        for _ in 0..14 {
            state.graveyard.add_card(Card::Sorcery(SpellCard {
                base: BaseCard {
                    name: "Bitter Triumph".to_string(),
                    mana_cost: ManaCost::default(),
                    mana_value: 2,
                },
                abilities: vec![],
            }));
        }

        // Spider-Man 4 + near Overlord 5 + DMM 1 + bonus 2*(14/7)=4 = 14
        assert_eq!(calculate_next_turn_damage(&state), 14);
    }

    #[test]
    fn test_calculate_combo_damage_multiple_terrors() {
        let mut state = GameState::new();
        state.opponent_life = 20;

        // Add 2 Terrors to battlefield
        for _ in 0..2 {
            let terror = Card::Creature(CreatureCard {
                base: BaseCard {
                    name: "Terror of the Peaks".to_string(),
                    mana_cost: ManaCost::default(),
                    mana_value: 4,
                },
                power: 3,
                toughness: 3,
                is_legendary: false,
                creature_types: vec![],
                abilities: vec![],
                impending_cost: None,
                impending_counters: None,
            });

            let permanent = Permanent::new(terror, 1);
            state.battlefield.add_permanent(permanent);
        }

        // Add Bringer to graveyard
        let bringer = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: ManaCost::default(),
                mana_value: 6,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        state.graveyard.add_card(bringer);

        let damage = calculate_combo_damage(&state);
        // Each Terror triggers for the Spider-Man copy (4, user rule) +
        // Bringer (6) = 10 per Terror; 2 Terrors = 20 damage
        assert_eq!(damage, 20);
    }
}


#[cfg(test)]
mod new_card_tests {
    use super::*;
    use crate::card::CardDatabase;
    use crate::game::zones::Permanent;
    use crate::rng::GameRng;

    fn load_db() -> CardDatabase {
        CardDatabase::from_file("cards.json").expect("Failed to load cards")
    }

    #[test]
    fn test_theorix_charm_mills_3_and_draws_1() {
        let db = load_db();
        let mut state = GameState::new();
        for _ in 0..5 {
            state.library.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        }
        let charm = db.get_card("Theorix Charm").expect("Theorix Charm");
        let mut rng = GameRng::new(Some(1));
        cast_spell(&mut state, &charm, &db, false, &mut rng).expect("cast should resolve");
        assert_eq!(state.graveyard.cards().iter().filter(|c| c.name() == "Bringer of the Last Gift").count(), 3);
        assert_eq!(state.hand.cards().len(), 1);
        assert_eq!(state.library.cards().len(), 1);
    }

    #[test]
    fn test_gloomlake_verge_counts_passage_as_swamp() {
        let db = load_db();
        let mut state = GameState::new();
        let gloomlake = Permanent::new(db.get_card("Gloomlake Verge").expect("Gloomlake"), 1);
        state.battlefield.add_permanent(gloomlake.clone());

        // Alone: only {U}
        let f = crate::game::mana::can_tap_for_mana(&gloomlake, &state, None);
        assert!(f.has_blue() && !f.has_black());

        // With a Multiversal Passage that chose Swamp: {U} or {B}
        let mut passage = Permanent::new(db.get_card("Multiversal Passage").expect("Passage"), 1);
        passage.chosen_basic_type = Some("B".to_string());
        state.battlefield.add_permanent(passage);
        let f = crate::game::mana::can_tap_for_mana(&gloomlake, &state, None);
        assert!(f.has_blue() && f.has_black(), "Passage set to Swamp turns on Gloomlake's B");
    }

    #[test]
    fn test_passage_ignores_terror_red_and_printed_colors() {
        let db = load_db();
        let mut state = GameState::new();
        // Starting Town lists every color, and Terror of the Peaks ({3}{R}{R}) is
        // in hand. Before the fix, Passage became a Mountain.
        state.battlefield.add_permanent(Permanent::new(db.get_card("Starting Town").expect("Town"), 1));
        state.hand.add_card(db.get_card("Terror of the Peaks").expect("Terror"));
        state.hand.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        let choice = choose_passage_color(&state);
        assert!(choice == "U" || choice == "B", "chose {}", choice);
    }

    #[test]
    fn test_passage_turns_on_gloomlake() {
        let db = load_db();
        let mut state = GameState::new();
        state.battlefield.add_permanent(Permanent::new(db.get_card("Gloomlake Verge").expect("Gloomlake"), 1));
        assert_eq!(choose_passage_color(&state), "B");
    }

    #[test]
    fn test_dark_matter_manipulator_etb_mills_3() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 1;
        for _ in 0..5 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }

        let dmm = db.get_card("Dark Matter Manipulator").expect("DMM");
        let mut perm = Permanent::new(dmm, 1);
        state.battlefield.add_permanent(perm.clone());

        let mut rng = GameRng::new(Some(7));
        process_etb_triggers_verbose(&mut state, &mut perm, &db, false, &mut rng)
            .expect("ETB should resolve");

        assert_eq!(state.graveyard.cards().len(), 3, "DMM should mill 3");
        assert_eq!(state.library.size(), 2);
    }

    #[test]
    fn test_does_machines_loots_and_discards_bringer() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 1;
        for _ in 0..6 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }
        state.hand.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));

        let does_machines = db.get_card("Does Machines").expect("Does Machines");
        let mut rng = GameRng::new(Some(3));
        cast_spell(&mut state, &does_machines, &db, false, &mut rng)
            .expect("cast should resolve");

        // Mill 2, draw 2, then discard 2 (Bringer is the top discard priority)
        assert!(
            state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift"),
            "Bringer should be discarded to the graveyard"
        );
        assert_eq!(state.graveyard.cards().len(), 4, "2 milled + 2 discarded");
        assert_eq!(state.hand.size(), 1, "1 + 2 drawn - 2 discarded");
        assert!(state.battlefield.permanents().iter().any(|p| p.card.name() == "Does Machines"));
    }

    #[test]
    fn test_bitter_triumph_discards_bringer() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 2;
        state.hand.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        state.hand.add_card(db.get_card("Forest").expect("Forest"));

        let bitter = db.get_card("Bitter Triumph").expect("Bitter Triumph");
        let mut rng = GameRng::new(Some(3));
        cast_spell(&mut state, &bitter, &db, false, &mut rng).expect("cast should resolve");

        assert!(
            state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift"),
            "Bringer should be discarded as the additional cost"
        );
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Bitter Triumph"));
        assert_eq!(state.hand.size(), 1, "Forest stays in hand");
        assert_eq!(state.life, 20, "No life paid when a discard is available");
    }

    #[test]
    fn test_marang_etb_bounces_up_to_two_value_permanents() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;

        // Does Machines and an impending Overlord are the bounce targets
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Does Machines").expect("Does Machines"), 1));
        let mut overlord = Permanent::new(
            db.get_card("Overlord of the Balemurk").expect("Overlord"), 2);
        overlord.add_counter(CounterType::Time, 4);
        state.battlefield.add_permanent(overlord);

        let marang = db.get_card("Marang River Regent").expect("Marang");
        let mut perm = Permanent::new(marang, 3);
        state.battlefield.add_permanent(perm.clone());

        let mut rng = GameRng::new(Some(9));
        process_etb_triggers_verbose(&mut state, &mut perm, &db, false, &mut rng)
            .expect("ETB should resolve");

        // Both value permanents are back in hand for a recast
        assert!(state.hand.cards().iter().any(|c| c.name() == "Does Machines"));
        assert!(state.hand.cards().iter().any(|c| c.name() == "Overlord of the Balemurk"));
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Does Machines"));
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Overlord of the Balemurk"));
        // Marang itself stays
        assert!(state.battlefield.permanents().iter().any(|p| p.card.name() == "Marang River Regent"));
    }

    #[test]
    fn test_bitter_triumph_destroys_own_creature_for_ardyn() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;

        // Ardyn is in the graveyard with no other creature: Starscourge needs fodder
        state.graveyard.add_card(db.get_card("Ardyn, the Usurper").expect("Ardyn"));
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Dark Matter Manipulator").expect("DMM"), 2));

        let bitter = db.get_card("Bitter Triumph").expect("Bitter Triumph");
        let mut rng = GameRng::new(Some(9));
        cast_spell(&mut state, &bitter, &db, false, &mut rng).expect("cast should resolve");

        // Hand is empty, so the additional cost is 3 life
        assert_eq!(state.life, 17, "Paid 3 life with nothing good to discard");
        // Our own creature was destroyed into the graveyard
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Dark Matter Manipulator"));
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Dark Matter Manipulator"));
    }

    #[test]
    fn test_bitter_triumph_cost_choice_life_vs_discard() {
        let db = load_db();
        let bitter = db.get_card("Bitter Triumph").expect("Bitter Triumph");

        // Case 1: only keeper cards in hand -> pay 3 life, keep Spider-Man
        let mut state = GameState::new();
        state.turn = 2;
        state.hand.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        let mut rng = GameRng::new(Some(9));
        cast_spell(&mut state, &bitter, &db, false, &mut rng).expect("cast should resolve");
        assert_eq!(state.life, 17, "Pays life to keep Spider-Man");
        assert!(state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man"));

        // Case 2: a blank in hand (an excess land) -> discard it, keep the life
        let mut state = GameState::new();
        state.turn = 2;
        for _ in 0..3 {
            state.hand.add_card(db.get_card("Forest").expect("Forest"));
        }
        cast_spell(&mut state, &bitter, &db, false, &mut rng).expect("cast should resolve");
        assert_eq!(state.life, 20, "Discards the blank instead of paying life");
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Forest"));

        // Case 3: life at 6 or less -> never pay down to 3 or less, discard instead
        let mut state = GameState::new();
        state.turn = 2;
        state.life = 5;
        state.hand.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        cast_spell(&mut state, &bitter, &db, false, &mut rng).expect("cast should resolve");
        assert_eq!(state.life, 5, "Does not pay life at 5 life");
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Superior Spider-Man"));
    }

    #[test]
    fn test_beseech_bargains_enchantment_creature_over_does_machines() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;
        state.opponent_life = 5;

        // Lethal combo in the graveyard, Spider-Man in the library
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        state.graveyard.add_card(db.get_card("Terror of the Peaks").expect("Terror"));
        state.library.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        for _ in 0..4 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }

        // Both fodder kinds on the battlefield
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Does Machines").expect("Does Machines"), 2));
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Fear of Missing Out").expect("FOMO"), 2));

        let beseech = db.get_card("Beseech the Mirror").expect("Beseech");
        let mut rng = GameRng::new(Some(11));
        cast_spell(&mut state, &beseech, &db, false, &mut rng).expect("cast should resolve");

        // The free cast happened (bargain was used)
        let spidey = state.battlefield.permanents().iter()
            .find(|p| p.card.name() == "Superior Spider-Man")
            .expect("Spider-Man should be on the battlefield");
        assert_eq!(spidey.is_copy_of, Some("Bringer of the Last Gift"));

        // The enchantment creature was the bargain, not Does Machines
        assert!(
            state.battlefield.permanents().iter().any(|p| p.card.name() == "Does Machines"),
            "Does Machines must stay on the battlefield"
        );
        assert!(state.graveyard.cards().iter().all(|c| c.name() != "Does Machines"));
        // FOMO went to the graveyard and the mass reanimate returned it
        let fomo = state.battlefield.permanents().iter()
            .find(|p| p.card.name() == "Fear of Missing Out")
            .expect("FOMO should be back on the battlefield");
        assert_eq!(fomo.turn_entered, 3, "FOMO re-entered this turn via mass reanimate");
    }

    #[test]
    fn test_beseech_free_casts_coil_and_catch_without_combo_target() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;

        // No combo target in the graveyard; Marang in the library with cards to draw
        state.library.add_card(db.get_card("Marang River Regent").expect("Marang"));
        for _ in 0..6 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }
        // Bringer in hand: the discard should bin it
        state.hand.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Does Machines").expect("Does Machines"), 2));

        let hand_before = state.hand.cards().len();
        let library_before = state.library.cards().len();
        let beseech = db.get_card("Beseech the Mirror").expect("Beseech");
        let mut rng = GameRng::new(Some(5));
        cast_spell(&mut state, &beseech, &db, false, &mut rng).expect("cast should resolve");

        // Bargained Does Machines, drew 3, discarded 1 (net +2 cards in hand)
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Does Machines"));
        assert_eq!(state.hand.cards().len(), hand_before + 2);
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift"),
            "the discard should put Bringer into the graveyard");
        // Marang was shuffled back into the library, not put in hand
        assert!(state.hand.cards().iter().all(|c| c.name() != "Marang River Regent"));
        assert!(state.library.cards().iter().any(|c| c.name() == "Marang River Regent"));
        assert_eq!(state.library.cards().len(), library_before - 3);
    }

    #[test]
    fn test_beseech_bargains_grim_bauble_over_does_machines() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;
        state.opponent_life = 5;

        // Lethal combo in the graveyard, Spider-Man in the library
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        state.graveyard.add_card(db.get_card("Terror of the Peaks").expect("Terror"));
        state.library.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        for _ in 0..4 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }

        // No enchantment creature available, and no mana is free for the
        // Bauble's surveil: the Bauble outranks Does Machines
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Does Machines").expect("Does Machines"), 2));
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Grim Bauble").expect("Grim Bauble"), 2));

        let beseech = db.get_card("Beseech the Mirror").expect("Beseech");
        let mut rng = GameRng::new(Some(11));
        cast_spell(&mut state, &beseech, &db, false, &mut rng).expect("cast should resolve");

        // The free cast happened (bargain was used)
        let spidey = state.battlefield.permanents().iter()
            .find(|p| p.card.name() == "Superior Spider-Man")
            .expect("Spider-Man should be on the battlefield");
        assert_eq!(spidey.is_copy_of, Some("Bringer of the Last Gift"));

        // Grim Bauble was the bargain, not Does Machines
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Grim Bauble"));
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Grim Bauble"));
        assert!(
            state.battlefield.permanents().iter().any(|p| p.card.name() == "Does Machines"),
            "Does Machines must stay on the battlefield"
        );
    }

    #[test]
    fn test_beseech_keeps_bauble_for_surveil_when_mana_is_free() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;
        state.opponent_life = 5;

        // Lethal combo in the graveyard, Spider-Man in the library
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        state.graveyard.add_card(db.get_card("Terror of the Peaks").expect("Terror"));
        state.library.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        for _ in 0..4 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }

        // Spare mana pays the Bauble's surveil this turn: the either/or choice
        // keeps the Bauble and bargains Does Machines instead
        let swamp = db.get_card("Swamp").expect("Swamp");
        for _ in 0..3 {
            state.battlefield.add_permanent(Permanent::new(swamp.clone(), 1));
        }
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Does Machines").expect("Does Machines"), 2));
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Grim Bauble").expect("Grim Bauble"), 2));

        let beseech = db.get_card("Beseech the Mirror").expect("Beseech");
        let mut rng = GameRng::new(Some(11));
        cast_spell(&mut state, &beseech, &db, false, &mut rng).expect("cast should resolve");

        // Does Machines was the bargain; the Bauble stays for its surveil
        assert!(state.battlefield.permanents().iter().any(|p| p.card.name() == "Grim Bauble"));
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Does Machines"));
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Does Machines"));
    }

    #[test]
    fn test_beseech_bargained_casts_spider_man_free() {
        let db = load_db();
        let mut state = GameState::new();
        state.turn = 3;
        state.opponent_life = 5;

        // Combo is ready in the graveyard and lethal (estimated 6 damage)
        state.graveyard.add_card(db.get_card("Bringer of the Last Gift").expect("Bringer"));
        state.graveyard.add_card(db.get_card("Terror of the Peaks").expect("Terror"));

        // Bargain fodder on the battlefield
        state.battlefield.add_permanent(Permanent::new(
            db.get_card("Does Machines").expect("Does Machines"), 2));

        // Spider-Man is in the library
        state.library.add_card(db.get_card("Superior Spider-Man").expect("Spidey"));
        for _ in 0..4 {
            state.library.add_card(db.get_card("Forest").expect("Forest"));
        }

        let beseech = db.get_card("Beseech the Mirror").expect("Beseech");
        let mut rng = GameRng::new(Some(11));
        cast_spell(&mut state, &beseech, &db, false, &mut rng).expect("cast should resolve");

        // Does Machines was sacrificed for bargain
        assert!(state.battlefield.permanents().iter().all(|p| p.card.name() != "Does Machines"));
        assert!(state.graveyard.cards().iter().any(|c| c.name() == "Does Machines"));

        // Spider-Man was cast for free and copied Bringer (combo fired)
        let spidey = state.battlefield.permanents().iter()
            .find(|p| p.card.name() == "Superior Spider-Man")
            .expect("Spider-Man should be on the battlefield");
        assert_eq!(spidey.is_copy_of, Some("Bringer of the Last Gift"));

        // Terror was mass-reanimated
        assert!(state.battlefield.permanents().iter().any(|p| p.card.name() == "Terror of the Peaks"));
        assert!(state.library.cards().iter().all(|c| c.name() != "Superior Spider-Man"));
    }
}
