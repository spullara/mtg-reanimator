use crate::card::{Card, CardDatabase, CardType, LandSubtype, ManaColor, ManaCost};
use crate::game::state::GameState;
use crate::game::zones::{CounterType, Permanent};
use crate::simulation::decisions::DecisionEngine;

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
        LandSubtype::Utility => {
            // Verge lands: enter untapped if revealed land type
            // Simplified: always enter untapped
            enters_tapped = false;
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

/// Choose basic land type for Multiversal Passage
/// Priority: Fill missing colors for castable spells
fn choose_passage_color(state: &GameState) -> String {
    // Check what colors we currently have access to from untapped lands
    let mut has_blue = false;
    let mut has_black = false;
    let mut has_green = false;

    for perm in state.battlefield.permanents() {
        if perm.tapped {
            continue;
        }
        if let Card::Land(land) = &perm.card {
            for color in &land.colors {
                match color {
                    ManaColor::Blue => has_blue = true,
                    ManaColor::Black => has_black = true,
                    ManaColor::Green => has_green = true,
                    _ => {}
                }
            }
        }
    }

    // Check what colors we need for spells in hand
    let mut needs_blue = false;
    let mut needs_black = false;
    let mut needs_green = false;

    for card in state.hand.cards() {
        let cost = card_mana_cost(card);
        if cost.blue > 0 {
            needs_blue = true;
        }
        if cost.black > 0 {
            needs_black = true;
        }
        if cost.green > 0 {
            needs_green = true;
        }
    }

    // Priority: Fill missing colors for castable spells
    if needs_green && !has_green {
        return "G".to_string();
    } else if needs_blue && !has_blue {
        return "U".to_string();
    } else if needs_black && !has_black {
        return "B".to_string();
    } else if !has_blue {
        // Default: prioritize blue for Spider-Man and Kiora
        return "U".to_string();
    } else if !has_black {
        return "B".to_string();
    } else if !has_green {
        return "G".to_string();
    }

    // Fallback
    "U".to_string()
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
    _db: &CardDatabase,
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
                    permanent.is_copy_of = Some("Bringer of the Last Gift".to_string());

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
                    permanent.is_copy_of = Some("Ardyn, the Usurper".to_string());

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
                    // Priority: Overlord of the Balemurk > Kiora > Town Greeter
                    let mill_creature = state.graveyard.cards().iter()
                        .position(|c| c.name() == "Overlord of the Balemurk")
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
                        permanent.is_copy_of = Some(creature_name.clone());

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
                    } else if verbose {
                        println!("    Spider-Man enters as a 4/4 (no good copy target, but have another Spider-Man)");
                    }
                } else if verbose {
                    println!("    Spider-Man enters as a 4/4 (no good copy target)");
                }
            }
            _ => {} // Other abilities handled elsewhere
        }
    }

    Ok(())
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

    let spider_man_copy_target: Option<String> = if spider_man_being_reanimated {
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
            Some("Terror of the Peaks".to_string())
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
            if let Some(ref copy_target) = spider_man_copy_target {
                perm.is_copy_of = Some(copy_target.clone());
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
            _ => {}
        }
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

/// Resolve Kiora's ETB ability: draw 2, discard 2
///
/// EXACT LOGIC FROM TYPESCRIPT:
/// - Draw 2 cards first
/// - Then discard 2 cards with 5-priority system:
///   1. Bringer of the Last Gift
///   2. Terror of the Peaks
///   3. Ardyn, the Usurper (8 mana - want to reanimate, not cast)
///   4. Excess lands (only if > 2 lands in hand)
///   5. Last card in hand
/// - Each discard iteration searches for the best card independently
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

    // Discard 2 - prioritize discarding Bringer/Terror
    let mut discarded: Vec<String> = Vec::new();
    for _ in 0..2 {
        if state.hand.size() == 0 {
            break;
        }

        // Find best card to discard
        let mut to_discard_idx: Option<usize> = None;

        // Priority 1: Bringer of the Last Gift
        if to_discard_idx.is_none() {
            to_discard_idx = state.hand.cards()
                .iter()
                .position(|c| c.name() == "Bringer of the Last Gift");
        }

        // Priority 2: Terror of the Peaks
        if to_discard_idx.is_none() {
            to_discard_idx = state.hand.cards()
                .iter()
                .position(|c| c.name() == "Terror of the Peaks");
        }

        // Priority 3: Ardyn, the Usurper (8 mana - want to reanimate, not cast)
        if to_discard_idx.is_none() {
            to_discard_idx = state.hand.cards()
                .iter()
                .position(|c| c.name() == "Ardyn, the Usurper");
        }

        // Priority 4: Excess lands (only if > 2 lands in hand)
        if to_discard_idx.is_none() {
            let lands: Vec<usize> = state.hand.cards()
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Card::Land(_)))
                .map(|(i, _)| i)
                .collect();
            if lands.len() > 2 {
                // Take the last land
                to_discard_idx = lands.last().copied();
            }
        }

        // Priority 5: Last card in hand
        if to_discard_idx.is_none() {
            to_discard_idx = Some(state.hand.size() - 1);
        }

        // Discard the card
        if let Some(idx) = to_discard_idx {
            if let Some(card) = state.hand.remove_card(idx) {
                let card_name = card.name().to_string();
                state.graveyard.add_card(card);
                discarded.push(card_name);
            }
        }
    }

    if verbose {
        println!("    Kiora ETB: discarded {}", discarded.join(", "));
    }
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
    if !has_spider_man {
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

    // Spider-Man copies Bringer (power 6), and Bringer (the copied one) is exiled
    const BRINGER_POWER: u32 = 6;

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
        terror_damage += BRINGER_POWER * terrors_on_battlefield;
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

/// Check if casting the combo NOW would be lethal
pub fn is_combo_lethal(state: &GameState) -> bool {
    let expected_damage = calculate_combo_damage(state);
    expected_damage >= state.opponent_life as u32
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
        // Terror on battlefield triggers for Spider-Man (6 power) + Bringer (6 power) = 12 damage
        assert_eq!(damage, 12);
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

    #[test]
    fn test_is_combo_lethal_true() {
        let mut state = GameState::new();
        state.opponent_life = 10;

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

        // Damage = 12 (Terror triggers for Spider-Man 6 + Bringer 6)
        // Opponent life = 10
        // 12 >= 10 = true
        assert!(is_combo_lethal(&state));
    }

    #[test]
    fn test_is_combo_lethal_false() {
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

        // Damage = 12 (Terror triggers for Spider-Man 6 + Bringer 6)
        // Opponent life = 20
        // 12 >= 20 = false
        assert!(!is_combo_lethal(&state));
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
        // Each Terror triggers for Spider-Man (6) + Bringer (6) = 12 per Terror
        // 2 Terrors = 24 damage
        assert_eq!(damage, 24);
    }
}

