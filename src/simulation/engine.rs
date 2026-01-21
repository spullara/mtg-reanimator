use crate::card::{Card, CardDatabase, ColorFlags, LandCard, LandSubtype, ManaColor};
use crate::game::state::GameState;
use crate::game::turns::{start_turn, draw_phase, upkeep_phase, end_phase, precombat_main_phase_start};
use crate::game::cards;
use crate::game::mana;
use crate::simulation::decisions::DecisionEngine;
use crate::rng::GameRng;
use crate::simulation::mulligan::resolve_mulligans;

/// Result of a single game simulation
#[derive(Debug, Clone)]
pub struct GameResult {
    /// Turn on which the game was won (None if didn't win by turn 20)
    pub win_turn: Option<u32>,
    /// First turn we had access to U, B, and G mana
    pub turn_with_ubg: Option<u32>,
}

/// Check if the game has been won
pub fn check_win_condition(state: &GameState) -> bool {
    state.opponent_life <= 0
}

/// Get available mana colors from battlefield lands as bitflags (no allocations)
/// Uses can_tap_for_mana to correctly handle conditional lands like Verge lands
#[inline]
fn get_available_colors(state: &GameState) -> ColorFlags {
    let mut colors = ColorFlags::new();

    for permanent in state.battlefield.permanents() {
        if matches!(permanent.card, Card::Land(_)) {
            // Use can_tap_for_mana to correctly evaluate conditional lands (Verge, Cavern, etc.)
            let land_colors = mana::can_tap_for_mana(permanent, state, None);
            colors.0 |= land_colors.0;
        }
    }

    colors
}

/// Check if Ardyn, the Usurper is on the battlefield
fn has_ardyn_on_battlefield(state: &GameState) -> bool {
    state.battlefield.permanents().iter().any(|p| {
        p.card.name() == "Ardyn, the Usurper"
            || p.is_copy_of.as_deref() == Some("Ardyn, the Usurper")
    })
}

/// Check if a permanent is a Demon (has "Demon" in creature_types or is a copy of a Demon)
fn is_demon(permanent: &crate::game::zones::Permanent) -> bool {
    match &permanent.card {
        Card::Creature(c) => c.creature_types.iter().any(|t| t == "Demon"),
        _ => false,
    }
}

/// Resolve Ardyn's Starscourge trigger: exile a creature from graveyard and create a 5/5 Demon token copy
fn resolve_starscourge(state: &mut GameState, verbose: bool) {
    // Find the best creature in graveyard to exile
    // Priority: high power creatures, especially reanimation targets like Bringer
    let mut best_idx: Option<usize> = None;
    let mut best_power: u32 = 0;

    for (idx, card) in state.graveyard.cards().iter().enumerate() {
        if let Card::Creature(c) = card {
            // Prioritize Bringer of the Last Gift and Terror of the Peaks
            let priority_boost = if c.base.name == "Bringer of the Last Gift" {
                100
            } else if c.base.name == "Terror of the Peaks" {
                50
            } else {
                0
            };
            let effective_power = c.power + priority_boost;

            if effective_power > best_power {
                best_power = effective_power;
                best_idx = Some(idx);
            }
        }
    }

    if let Some(idx) = best_idx {
        // Get the creature name before removing
        let creature_name = state.graveyard.cards()[idx].name().to_string();
        // Note: creature_power is not used since token is always 5/5, but keeping for reference
        let _creature_power = if let Card::Creature(c) = &state.graveyard.cards()[idx] {
            c.power
        } else {
            5
        };

        // Remove from graveyard and add to exile
        if let Some(card) = state.graveyard.remove_card(idx) {
            if verbose {
                println!("[Starscourge] Ardyn exiles {} from graveyard", card.name());
            }
            state.add_to_exile(card);
        }

        // Create a 5/5 Demon token copy of the exiled creature
        // The token has Demon creature type added so it benefits from Ardyn's abilities
        let token = Card::Creature(crate::card::CreatureCard {
            base: crate::card::types::BaseCard {
                name: format!("{} (Starscourge Token)", creature_name),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            power: 5,
            toughness: 5,
            is_legendary: false,
            creature_types: vec!["Demon".to_string()],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let mut perm = crate::game::zones::Permanent::new(token, state.turn);
        perm.is_copy_of = Some(creature_name.clone());

        state.battlefield.add_permanent(perm);

        if verbose {
            println!("[Starscourge] Created a 5/5 Demon token copy of {} (has haste from Ardyn)", creature_name);
        }

        // Trigger Terror of the Peaks if on battlefield (for the 5/5 token entering)
        let terror_count = state.battlefield.permanents().iter()
            .filter(|p| {
                p.card.name() == "Terror of the Peaks"
                    || p.is_copy_of.as_deref() == Some("Terror of the Peaks")
            })
            .count() as i32;

        if terror_count > 0 {
            let terror_damage = 5 * terror_count; // Token is 5/5
            state.opponent_life -= terror_damage;
            if verbose {
                println!("[Terror] {} damage from Starscourge token entering (5 power x {} Terror(s))",
                    terror_damage, terror_count);
            }
        }
    }
}

/// Simulate combat phase: declare attackers and deal damage
pub fn simulate_combat(state: &mut GameState, verbose: bool) -> u32 {
    let mut total_damage = 0;

    // Check if Ardyn is on the battlefield (for haste and Starscourge)
    let ardyn_on_battlefield = has_ardyn_on_battlefield(state);

    // Resolve Starscourge trigger at beginning of combat (if Ardyn is on battlefield)
    if ardyn_on_battlefield {
        resolve_starscourge(state, verbose);
    }

    // Find eligible attackers (creatures without summoning sickness, not tapped)
    let mut attackers = Vec::new();
    let mut lifelink_damage = 0u32;

    for (idx, permanent) in state.battlefield.permanents().iter().enumerate() {
        // Must be a creature
        if !matches!(permanent.card, Card::Creature(_)) {
            continue;
        }

        // Check for impending counters (creature is still an enchantment)
        if permanent.get_counter(crate::game::zones::CounterType::Time) > 0 {
            continue;
        }

        // Check summoning sickness (entered before this turn)
        // Exception: Demons have haste if Ardyn is on battlefield
        let has_summoning_sickness = permanent.turn_entered >= state.turn;
        if has_summoning_sickness {
            let demon_with_haste = ardyn_on_battlefield && is_demon(permanent);
            if !demon_with_haste {
                continue;
            }
        }

        // Check if tapped
        if permanent.tapped {
            continue;
        }

        attackers.push(idx);
    }

    // Tap all attackers and calculate damage
    for idx in attackers {
        if let Some(permanent) = state.battlefield.permanents_mut().get_mut(idx) {
            permanent.tapped = true;

            // Get creature power
            if let Card::Creature(creature) = &permanent.card {
                let power = creature.power as u32;
                total_damage += power;

                // Track lifelink damage for Demons when Ardyn is present
                if ardyn_on_battlefield && creature.creature_types.iter().any(|t| t == "Demon") {
                    lifelink_damage += power;
                }
            }
        }
    }

    // Deal damage to opponent
    state.opponent_life -= total_damage as i32;

    // Gain life from lifelink
    if lifelink_damage > 0 {
        state.life += lifelink_damage as i32;
        if verbose {
            println!("[Combat] Gained {} life from Demon lifelink", lifelink_damage);
        }
    }

    if verbose && total_damage > 0 {
        println!("[Combat] {} damage dealt", total_damage);
    }

    total_damage
}

/// Execute a single turn: untap -> draw -> main -> combat -> end
pub fn execute_turn(state: &mut GameState, db: &CardDatabase, verbose: bool, rng: &mut crate::rng::GameRng) -> u32 {
    // Start turn: increment turn counter, untap, reset land drop
    start_turn(state);

    if verbose {
        println!("\n=== TURN {} ===", state.turn);
    }

    // Upkeep phase
    upkeep_phase(state);

    // Draw phase
    state.phase = crate::game::state::Phase::Draw;
    let hand_before = state.hand.size();
    draw_phase(state);

    if verbose {
        if state.hand.size() > hand_before {
            // Get the last card drawn
            if let Some(card) = state.hand.cards().last() {
                println!("[Draw] Drew: {}", card.name());
            }
        } else if state.turn == 1 && state.on_the_play {
            println!("[Draw] Skipped (on the play)");
        }
    }

    // Main phase 1: Play lands and cast spells
    state.phase = crate::game::state::Phase::Main1;

    // Precombat main phase start: advance saga counters and resolve chapters
    // Per MTG rules, saga lore counters are added at the beginning of precombat main phase
    precombat_main_phase_start(state, verbose);
    if verbose {
        let hand_names: Vec<&str> = state.hand.cards().iter().map(|c| c.name()).collect();
        println!("[Main 1] Hand: {}", hand_names.join(", "));
    }
    execute_main_phase(state, db, verbose, rng);

    // Combat phase
    state.phase = crate::game::state::Phase::Combat;
    let combat_damage = simulate_combat(state, verbose);

    // Main phase 2: Additional spell casting could happen here
    state.phase = crate::game::state::Phase::Main2;
    // For now, we don't do anything in main 2

    // End phase
    state.phase = crate::game::state::Phase::End;
    end_phase(state);

    if verbose {
        println!("[End of Turn {}]", state.turn);
        let battlefield_names: Vec<String> = state.battlefield.permanents()
            .iter()
            .map(|p| {
                let mut name = p.card.name().to_string();
                if let Some(copy_of) = &p.is_copy_of {
                    name.push_str(&format!(" (copy of {})", copy_of));
                }
                if let Some(time_counters) = p.counters.get(&crate::game::zones::CounterType::Time) {
                    name.push_str(&format!(" ({} time counters)", time_counters));
                }
                name
            })
            .collect();
        println!("  Battlefield: {}", if battlefield_names.is_empty() { "(empty)".to_string() } else { battlefield_names.join(", ") });

        let graveyard_names: Vec<&str> = state.graveyard.cards().iter().map(|c| c.name()).collect();
        println!("  Graveyard: {}", if graveyard_names.is_empty() { "(empty)".to_string() } else { graveyard_names.join(", ") });

        println!("  Opponent life: {}", state.opponent_life);
    }

    combat_damage
}

/// Get mana cost from a card
fn get_mana_cost(card: &Card) -> &crate::card::ManaCost {
    match card {
        Card::Land(c) => &c.base.mana_cost,
        Card::Creature(c) => &c.base.mana_cost,
        Card::Instant(c) => &c.base.mana_cost,
        Card::Sorcery(c) => &c.base.mana_cost,
        Card::Enchantment(c) => &c.base.mana_cost,
        Card::Saga(c) => &c.base.mana_cost,
    }
}

/// Port of TypeScript mainPhase function (lines 2211-2502)
/// Core game logic that determines what spells to cast and in what order
pub fn main_phase(state: &mut GameState, db: &CardDatabase, verbose: bool, rng: &mut crate::rng::GameRng) {
    // SPECIAL CASE: Turn 4 combo check
    // If we have Spider-Man in hand, Bringer in GY, and can get to 4 mana by playing a land,
    // play the land FIRST before casting any other spells!
    let has_spider_man = state.hand.cards().iter().any(|c| c.name() == "Superior Spider-Man");
    let has_bringer_in_gy = state.graveyard.cards().iter().any(|c| c.name() == "Bringer of the Last Gift");
    let current_mana = state.battlefield.permanents()
        .iter()
        .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
        .count() as u32;

    if has_spider_man && has_bringer_in_gy && current_mana == 3 && !state.land_played_this_turn {
        // Check if we have an untapped land to play
        let hand_cards = state.hand.cards().to_vec();
        if let Some(untapped_land_idx) = hand_cards.iter().position(|c| {
            if let Card::Land(land) = c {
                // Check if land enters untapped
                !land.enters_tapped && land.subtype != crate::card::LandSubtype::Fastland
            } else {
                false
            }
        }) {
            if let Some(untapped_land) = state.hand.remove_card(untapped_land_idx) {
                let land_name = untapped_land.name().to_string();
                let _ = cards::play_land(state, &untapped_land, verbose);
                if verbose {
                    println!("  [COMBO SETUP] Played {} first to enable turn 4 combo", land_name);
                }
            }
        }
    }

    // STEP 1: If we haven't played a land yet and have land-finding spells,
    // cast those FIRST to potentially find a better land
    // BUT: If we have Bringer/Terror in hand and can cast Kiora or Formidable Speaker, skip this step!
    // These are more important (discard Bringer to graveyard for the combo)
    let has_bringer_or_terror_in_hand = state.hand.cards().iter().any(|c| {
        c.name() == "Bringer of the Last Gift" || c.name() == "Terror of the Peaks"
    });
    let kiora_in_hand = state.hand.cards().iter().find(|c| c.name() == "Kiora, the Rising Tide");
    let formidable_speaker_in_hand = state.hand.cards().iter().find(|c| c.name() == "Formidable Speaker");

    // Check if we can cast Kiora now OR if we could cast it after playing an untapped land
    let could_cast_kiora_after_land_drop = || -> bool {
        let kiora = match kiora_in_hand {
            Some(k) => k,
            None => return false,
        };

        // Can cast now?
        if mana::can_cast_spell(kiora, state) {
            return true;
        }

        // If we've already played a land, no look-ahead needed
        if state.land_played_this_turn {
            return false;
        }

        // Check if playing an untapped land would enable Kiora
        let current_mana = state.battlefield.permanents().iter()
            .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
            .count() as u32;
        let kiora_cost = kiora.mana_value();

        // Would one more mana be enough?
        if current_mana + 1 < kiora_cost {
            return false;
        }

        // Helper: check if land enters tapped
        let land_enters_tapped = |land: &LandCard| -> bool {
            match land.subtype {
                LandSubtype::Fastland => {
                    let land_count = state.battlefield.permanents().iter()
                        .filter(|p| matches!(p.card, Card::Land(_)))
                        .count();
                    land_count >= 3
                }
                LandSubtype::Town => state.turn > 3,
                LandSubtype::Shock => state.life <= 2, // Enters tapped if we can't pay 2 life
                _ => land.enters_tapped,
            }
        };

        // Check if we have an untapped land that produces U (Kiora needs U)
        let has_untapped_land_with_u = state.hand.cards().iter().any(|c| {
            if let Card::Land(land) = c {
                if land_enters_tapped(land) {
                    return false;
                }
                // Check if land produces U
                land.colors.contains(&ManaColor::Blue)
            } else {
                false
            }
        });

        // Also check if we already have U available and just need an untapped land for mana count
        // Use can_tap_for_mana to correctly handle conditional lands like Verge lands
        let has_u_available = state.battlefield.permanents().iter().any(|p| {
            if p.tapped {
                return false;
            }
            if matches!(p.card, Card::Land(_)) {
                mana::can_tap_for_mana(p, state, None).has_blue()
            } else {
                false
            }
        });

        if has_u_available {
            // We have U, just need any untapped land for the mana count
            let has_any_untapped_land = state.hand.cards().iter().any(|c| {
                if let Card::Land(land) = c {
                    !land_enters_tapped(land)
                } else {
                    false
                }
            });
            return has_any_untapped_land;
        }

        // We need the new land to provide U
        has_untapped_land_with_u
    };

    let should_prioritize_discard_spell = has_bringer_or_terror_in_hand
        && (kiora_in_hand.is_some() || formidable_speaker_in_hand.is_some())
        && could_cast_kiora_after_land_drop();

    // SPECIAL CASE: Analyze the Pollen early casting
    // Cast it early (without evidence) if:
    // 1. We need a land for UBG color fixing, OR
    // 2. Getting to 4 mana enables combo next turn (have Spider-Man + Bringer in GY)
    let analyze_pollen_idx = state.hand.cards().iter()
        .position(|c| c.name() == "Analyze the Pollen");

    if let Some(pollen_idx) = analyze_pollen_idx {
        let can_cast_pollen = mana::can_cast_spell(&state.hand.cards()[pollen_idx], state);

        if can_cast_pollen {
            let available_colors = get_available_colors(state);
            let has_u = available_colors.has_blue();
            let has_b = available_colors.has_black();
            let has_g = available_colors.has_green();
            let has_all_colors = has_u && has_b && has_g;

            // Count untapped lands
            let untapped_lands = state.battlefield.permanents().iter()
                .filter(|p| matches!(p.card, Card::Land(_)) && !p.tapped)
                .count() as u32;

            // Check if we have the combo pieces ready
            let has_spider_man = state.hand.cards().iter()
                .any(|c| c.name() == "Superior Spider-Man");
            let has_bringer_in_gy = state.graveyard.cards().iter()
                .any(|c| c.name() == "Bringer of the Last Gift");

            // Check lands in hand for potential land drop
            let has_land_in_hand = state.hand.cards().iter()
                .any(|c| matches!(c, Card::Land(_)));

            // Condition 1: Need a basic land for UBG color fixing
            // (missing a color and don't have a land that provides it)
            let needs_color_fixing = !has_all_colors && {
                // Check if we have a land in hand that would fix our colors
                let hand_would_fix = state.hand.cards().iter()
                    .filter_map(|c| if let Card::Land(l) = c { Some(l) } else { None })
                    .any(|land| {
                        (!has_u && land.colors.iter().any(|c| *c == ManaColor::Blue)) ||
                        (!has_b && land.colors.iter().any(|c| *c == ManaColor::Black)) ||
                        (!has_g && land.colors.iter().any(|c| *c == ManaColor::Green))
                    });
                // If we don't have a land in hand that fixes, we should Analyze
                !hand_would_fix
            };

            // Condition 2: Getting to 4 mana enables combo next turn
            // We have Spider-Man + Bringer in GY, currently at 2 or 3 mana,
            // and with land drop + pollen land we'd have 4
            let enables_combo_next_turn = has_spider_man && has_bringer_in_gy && {
                // Current mana + land in hand + pollen land = 4?
                // If we have 2 lands and a land in hand, pollen would get us to 4
                // If we have 3 lands and no land in hand, pollen would get us to 4
                (untapped_lands == 2 && has_land_in_hand) ||
                (untapped_lands == 3 && !has_land_in_hand && !state.land_played_this_turn)
            };

            let should_cast_pollen_early = needs_color_fixing || enables_combo_next_turn;

            if should_cast_pollen_early {
                // Cast Analyze the Pollen early
                if let Some(card) = state.hand.remove_card(pollen_idx) {
                    let cost = get_mana_cost(&card);
                    if mana::tap_lands_for_cost(cost, state, None) {
                        if verbose {
                            if needs_color_fixing {
                                println!("  [Cast] Analyze the Pollen (color fixing - no evidence)");
                            } else {
                                println!("  [Cast] Analyze the Pollen (enabling combo - no evidence)");
                            }
                        }
                        let _ = cards::cast_spell(state, &card, db, verbose, rng);
                    } else {
                        // Put it back if we can't pay
                        state.hand.add_card(card);
                    }
                }
            }
        }
    }

    if !state.land_played_this_turn && !should_prioritize_discard_spell {
        let mut cast_any = true;

        while cast_any && !state.land_played_this_turn {
            cast_any = false;

            // Land-finding spells (from TypeScript LAND_FINDING_SPELLS)
            const LAND_FINDERS: &[&str] = &[
                "Cache Grab",
                "Dredger's Insight",
                "Town Greeter",
            ];

            // Find castable land-finding spells
            let mut castable_finders: Vec<(usize, &Card)> = state.hand.cards()
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    LAND_FINDERS.contains(&c.name()) && mana::can_cast_spell(c, state)
                })
                .collect();

            if !castable_finders.is_empty() {
                // Sort by mana value (cheaper first)
                castable_finders.sort_by_key(|(_, c)| c.mana_value());

                let (spell_idx, _spell) = castable_finders[0];
                let lands_before = state.hand.cards().iter().filter(|c| matches!(c, Card::Land(_))).count();

                // Remove from hand and cast
                if let Some(card) = state.hand.remove_card(spell_idx) {
                    let cost = get_mana_cost(&card);
                    if mana::tap_lands_for_cost(cost, state, None) {
                        let card_name = card.name().to_string();

                        // Handle creatures specially (add to battlefield and process ETB)
                        if matches!(&card, Card::Creature(_)) {
                            let _ = cards::cast_creature(state, &card, false);

                            // Process ETB triggers
                            let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                            if perm_idx < state.battlefield.permanents().len() {
                                let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                                let _ = cards::process_etb_triggers_verbose(state, &mut perm, db, verbose, rng);
                                state.battlefield.permanents_mut()[perm_idx] = perm;
                            }
                        } else {
                            let _ = cards::cast_spell(state, &card, db, verbose, rng);
                        }

                        if verbose {
                            println!("  [Cast] {}", card_name);
                        }
                        cast_any = true;

                        // Check if we found a land
                        let lands_after = state.hand.cards().iter().filter(|c| matches!(c, Card::Land(_))).count();
                        if lands_after > lands_before && verbose {
                            println!("  [Land-finder] Found a land");
                        }
                    } else {
                        // Put it back if we can't pay
                        state.hand.add_card(card);
                    }
                }
            } else {
                break; // No more land-finding spells
            }
        }
    }

    // STEP 2: Now play a land (possibly one we just found from milling)
    if !state.land_played_this_turn {
        let hand_cards = state.hand.cards().to_vec();
        let lands_in_hand: Vec<&Card> = hand_cards.iter()
            .filter(|c| matches!(c, Card::Land(_)))
            .collect();

        if !lands_in_hand.is_empty() {
            // Use DecisionEngine to choose the best land
            if let Some(land_idx) = DecisionEngine::choose_land_to_play(&hand_cards, state) {
                if let Some(card) = state.hand.remove_card(land_idx) {
                    let card_name = card.name().to_string();
                    let _ = cards::play_land(state, &card, verbose);

                    // DO NOT tap the land here - TypeScript taps lands DURING casting
                    // This allows can_cast_spell to correctly see the new untapped land

                    if verbose {
                        let last_perm = state.battlefield.permanents().last();
                        let tapped_str = if let Some(perm) = last_perm {
                            if perm.tapped { " (tapped)" } else { "" }
                        } else {
                            ""
                        };
                        println!("  [Land] {}{}", card_name, tapped_str);
                    }
                }
            }
        }
    }

    // STEP 3: Cast remaining spells
    let mut cast_any = true;
    while cast_any {
        cast_any = false;

        // Get game state for spell priorities
        let has_bringer_in_graveyard = state.graveyard.cards().iter()
            .any(|c| c.name() == "Bringer of the Last Gift");
        let has_bringer_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Bringer of the Last Gift");
        let has_terror_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Terror of the Peaks");

        // Check if the combo would be lethal
        let combo_is_lethal = has_bringer_in_graveyard && cards::is_combo_lethal(state);
        let has_spider_man_in_hand = state.hand.cards().iter()
            .any(|c| c.name() == "Superior Spider-Man");

        // Log when we're holding back the combo
        if verbose && has_bringer_in_graveyard && has_spider_man_in_hand && !combo_is_lethal {
            let expected_damage = cards::calculate_combo_damage(state);
            println!(
                "  [Waiting] Combo not lethal yet (expected: {} damage, need: {})",
                expected_damage, state.opponent_life
            );
        }

        // Get castable spells
        let mut castable_spells: Vec<(usize, &Card)> = state.hand.cards()
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                if matches!(c, Card::Land(_)) {
                    return false;
                }
                if !mana::can_cast_spell(c, state) {
                    return false;
                }

                // Spider-Man casting logic:
                // 1. If Bringer in graveyard and combo is lethal -> cast (THE COMBO!)
                // 2. If no Bringer in graveyard but have 2+ Spider-Man in hand AND
                //    a mill creature in graveyard -> cast to dig for Bringer
                if c.name() == "Superior Spider-Man" {
                    if has_bringer_in_graveyard {
                        // Only cast if combo would be lethal
                        if !combo_is_lethal {
                            return false; // Wait until it would kill
                        }
                    } else {
                        // No Bringer in graveyard - check if we should dig
                        let spider_man_count = state.hand.cards().iter()
                            .filter(|card| card.name() == "Superior Spider-Man")
                            .count();
                        let has_mill_creature_in_gy = state.graveyard.cards().iter()
                            .any(|card| matches!(card.name(),
                                "Overlord of the Balemurk" |
                                "Kiora, the Rising Tide" |
                                "Town Greeter"));

                        if spider_man_count < 2 || !has_mill_creature_in_gy {
                            return false; // Can't dig effectively
                        }
                        // Otherwise, allow casting to dig for Bringer
                    }
                }

                true
            })
            .collect();

        if castable_spells.is_empty() {
            break;
        }

        // Sort by priority
        castable_spells.sort_by(|a, b| {
            let (_, a_card) = a;
            let (_, b_card) = b;

            // Priority 1: Spider-Man if combo is lethal
            if combo_is_lethal {
                if a_card.name() == "Superior Spider-Man" {
                    return std::cmp::Ordering::Less;
                }
                if b_card.name() == "Superior Spider-Man" {
                    return std::cmp::Ordering::Greater;
                }
            }

            // Priority 2: Kiora or Formidable Speaker if Bringer/Terror in hand
            // (These can discard combo pieces to the graveyard)
            if has_bringer_in_hand || has_terror_in_hand {
                // Prefer Formidable Speaker slightly (cheaper at 3 mana vs Kiora's 3)
                // and it tutors for Spider-Man
                if a_card.name() == "Formidable Speaker" {
                    return std::cmp::Ordering::Less;
                }
                if b_card.name() == "Formidable Speaker" {
                    return std::cmp::Ordering::Greater;
                }
                if a_card.name() == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Less;
                }
                if b_card.name() == "Kiora, the Rising Tide" {
                    return std::cmp::Ordering::Greater;
                }
            }

            // Priority 3: Mill spells
            let mill_spells = vec![
                "Cache Grab",
                "Dredger's Insight",
                "Town Greeter",
                "Overlord of the Balemurk",
            ];
            let a_is_mill = mill_spells.contains(&a_card.name());
            let b_is_mill = mill_spells.contains(&b_card.name());
            if a_is_mill && !b_is_mill {
                return std::cmp::Ordering::Less;
            }
            if b_is_mill && !a_is_mill {
                return std::cmp::Ordering::Greater;
            }

            // Priority 4: Awaken the Honored Dead
            if a_card.name() == "Awaken the Honored Dead" && !b_is_mill {
                return std::cmp::Ordering::Less;
            }
            if b_card.name() == "Awaken the Honored Dead" && !a_is_mill {
                return std::cmp::Ordering::Greater;
            }

            // Priority 5: Cheaper spells
            a_card.mana_value().cmp(&b_card.mana_value())
        });

        if !castable_spells.is_empty() {
            let (spell_idx, _spell) = castable_spells[0];

            if let Some(card) = state.hand.remove_card(spell_idx) {
                let card_name = card.name().to_string();

                // Get for_creature for Cavern of Souls handling and impending check
                let for_creature = match &card {
                    Card::Creature(c) => Some(c),
                    _ => None,
                };

                // Determine if we should use impending cost
                // For creatures with impending, prefer impending (it's cheaper and triggers immediately)
                let (use_impending, cost) = if let Some(creature) = for_creature {
                    if let Some(impending_cost) = &creature.impending_cost {
                        // Prefer impending when we can afford it
                        if mana::can_afford_cost(impending_cost, state, for_creature) {
                            (true, impending_cost.clone())
                        } else {
                            (false, get_mana_cost(&card).clone())
                        }
                    } else {
                        (false, get_mana_cost(&card).clone())
                    }
                } else {
                    (false, get_mana_cost(&card).clone())
                };

                if mana::tap_lands_for_cost(&cost, state, for_creature) {
                    match &card {
                        Card::Creature(_) => {
                            let _ = cards::cast_creature(state, &card, use_impending);

                            // Process ETB triggers
                            let perm_idx = state.battlefield.permanents().len().saturating_sub(1);
                            if perm_idx < state.battlefield.permanents().len() {
                                let mut perm = state.battlefield.permanents_mut()[perm_idx].clone();
                                let _ = cards::process_etb_triggers_verbose(state, &mut perm, db, verbose, rng);
                                state.battlefield.permanents_mut()[perm_idx] = perm;
                            }

                            if verbose {
                                if use_impending {
                                    println!("  [Cast] {} (impending)", card_name);
                                } else {
                                    println!("  [Cast] {}", card_name);
                                }
                            }
                        }
                        Card::Land(_) => {
                            let _ = cards::play_land(state, &card, verbose);
                            if verbose {
                                println!("  [Land] {}", card_name);
                            }
                        }
                        Card::Instant(_) | Card::Sorcery(_) | Card::Enchantment(_) | Card::Saga(_) => {
                            let _ = cards::cast_spell(state, &card, db, verbose, rng);
                            if verbose {
                                println!("  [Cast] {}", card_name);
                            }
                        }
                    }

                    cast_any = true;
                } else {
                    // Put it back if we can't pay
                    state.hand.add_card(card);
                }
            }
        }
    }
}

/// Execute main phase: play lands and cast spells
fn execute_main_phase(state: &mut GameState, db: &CardDatabase, verbose: bool, rng: &mut crate::rng::GameRng) {
    // DO NOT tap lands here - TypeScript taps lands DURING casting, not before
    // This means can_cast_spell checks untapped lands, and cast_spell taps them
    main_phase(state, db, verbose, rng);
}

/// Run a complete game simulation
pub fn run_game(
    deck: &[Card],
    seed: u64,
    _db: &CardDatabase,
    verbose: bool,
) -> GameResult {
    let mut rng = GameRng::new(Some(seed));

    // Initialize game state
    let mut state = GameState::new();

    // Determine if on play or draw (50/50) - BEFORE shuffling to match TypeScript RNG sequence
    state.on_the_play = rng.random() < 0.5;

    // Shuffle deck into library
    let mut shuffled_deck = deck.to_vec();
    rng.shuffle(&mut shuffled_deck);
    for card in shuffled_deck {
        state.library.add_card(card);
    }

    // Mulligan phase: resolve mulligans to get opening hand
    let mut library_cards = Vec::new();
    for _ in 0..state.library.size() {
        if let Some(card) = state.library.draw() {
            library_cards.push(card);
        }
    }

    let opening_hand = resolve_mulligans(&mut library_cards, &mut rng);

    // Put remaining cards back in library
    for card in library_cards {
        state.library.add_card(card);
    }

    // Add opening hand to hand
    for card in opening_hand.clone() {
        state.hand.add_card(card);
    }

    // Print game start info if verbose
    if verbose {
        println!("=== Game Start (seed: {}) ===", seed);
        println!("{}", if state.on_the_play { "On the play" } else { "On the draw" });
        println!("Opening hand ({} cards):", opening_hand.len());
        for card in &opening_hand {
            println!("  - {}", card.name());
        }
    }
    
    // Game loop
    let max_turns = 20u32;
    let mut turn_with_ubg = None;

    while state.turn < max_turns && !check_win_condition(&state) {
        // Execute turn
        execute_turn(&mut state, _db, verbose, &mut rng);

        // Track when all colors become available
        if turn_with_ubg.is_none() {
            let colors = get_available_colors(&state);
            if colors.has_blue() && colors.has_black() && colors.has_green() {
                turn_with_ubg = Some(state.turn);
            }
        }
    }
    
    GameResult {
        win_turn: if check_win_condition(&state) { Some(state.turn) } else { None },
        turn_with_ubg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::types::{BaseCard, CreatureCard, LandCard};

    #[test]
    fn test_check_win_condition_false() {
        let state = GameState::new();
        assert!(!check_win_condition(&state));
    }

    #[test]
    fn test_check_win_condition_true() {
        let mut state = GameState::new();
        state.opponent_life = 0;
        assert!(check_win_condition(&state));
    }

    #[test]
    fn test_check_win_condition_negative() {
        let mut state = GameState::new();
        state.opponent_life = -5;
        assert!(check_win_condition(&state));
    }

    #[test]
    fn test_get_available_colors_empty() {
        let state = GameState::new();
        let colors = get_available_colors(&state);
        assert!(colors.is_empty());
    }

    #[test]
    fn test_get_available_colors_with_lands() {
        use crate::card::{LandSubtype, ManaColor};

        let mut state = GameState::new();

        // Add a forest (green land)
        let forest = Card::Land(LandCard {
            base: BaseCard {
                name: "Forest".to_string(),
                mana_cost: Default::default(),
                mana_value: 0,
            },
            subtype: LandSubtype::Basic,
            colors: vec![ManaColor::Green],
            enters_tapped: false,
            has_surveil: false,
            surveil_amount: 0,
        });

        let permanent = crate::game::zones::Permanent::new(forest, 1);
        state.battlefield.add_permanent(permanent);

        let colors = get_available_colors(&state);
        assert!(colors.has_green());
    }

    #[test]
    fn test_simulate_combat_no_creatures() {
        let mut state = GameState::new();
        let damage = simulate_combat(&mut state, false);
        assert_eq!(damage, 0);
        assert_eq!(state.opponent_life, 20);
    }

    #[test]
    fn test_simulate_combat_with_creature() {
        let mut state = GameState::new();
        state.turn = 2; // Avoid summoning sickness
        
        // Add a creature to battlefield
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: Default::default(),
                mana_value: 1,
            },
            power: 3,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });
        
        let permanent = crate::game::zones::Permanent::new(creature, 1);
        state.battlefield.add_permanent(permanent);

        let damage = simulate_combat(&mut state, false);
        assert_eq!(damage, 3);
        assert_eq!(state.opponent_life, 17);
    }

    #[test]
    fn test_simulate_combat_summoning_sickness() {
        let mut state = GameState::new();
        state.turn = 1;

        // Add a creature that entered this turn (has summoning sickness)
        let creature = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: Default::default(),
                mana_value: 1,
            },
            power: 3,
            toughness: 2,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let permanent = crate::game::zones::Permanent::new(creature, 1);
        state.battlefield.add_permanent(permanent);

        let damage = simulate_combat(&mut state, false);
        assert_eq!(damage, 0); // Can't attack due to summoning sickness
        assert_eq!(state.opponent_life, 20);
    }

    #[test]
    fn test_demon_haste_with_ardyn() {
        let mut state = GameState::new();
        state.turn = 1;

        // Add Ardyn to battlefield (entered on a previous turn)
        let ardyn = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Ardyn, the Usurper".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 4,
            toughness: 4,
            is_legendary: true,
            creature_types: vec!["Elder".to_string(), "Human".to_string(), "Noble".to_string()],
            abilities: vec!["gives_demons_haste".to_string()],
            impending_cost: None,
            impending_counters: None,
        });

        let ardyn_perm = crate::game::zones::Permanent::new(ardyn, 0); // Entered turn 0
        state.battlefield.add_permanent(ardyn_perm);

        // Add a Demon that entered this turn (has summoning sickness normally)
        let demon = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec!["Vampire".to_string(), "Demon".to_string()],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let demon_perm = crate::game::zones::Permanent::new(demon, 1); // Entered this turn
        state.battlefield.add_permanent(demon_perm);

        let damage = simulate_combat(&mut state, false);
        // Demon should attack with haste (6) + Ardyn can attack (4) = 10
        assert_eq!(damage, 10);
        assert_eq!(state.opponent_life, 10);
    }

    #[test]
    fn test_demon_no_haste_without_ardyn() {
        let mut state = GameState::new();
        state.turn = 1;

        // Add a Demon that entered this turn (has summoning sickness)
        let demon = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec!["Vampire".to_string(), "Demon".to_string()],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let demon_perm = crate::game::zones::Permanent::new(demon, 1); // Entered this turn
        state.battlefield.add_permanent(demon_perm);

        let damage = simulate_combat(&mut state, false);
        // Demon can't attack without Ardyn (summoning sickness)
        assert_eq!(damage, 0);
        assert_eq!(state.opponent_life, 20);
    }

    #[test]
    fn test_lifelink_with_ardyn() {
        let mut state = GameState::new();
        state.turn = 2;

        // Add Ardyn to battlefield
        let ardyn = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Ardyn, the Usurper".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 4,
            toughness: 4,
            is_legendary: true,
            creature_types: vec!["Elder".to_string(), "Human".to_string(), "Noble".to_string()],
            abilities: vec!["gives_demons_lifelink".to_string()],
            impending_cost: None,
            impending_counters: None,
        });

        let ardyn_perm = crate::game::zones::Permanent::new(ardyn, 1);
        state.battlefield.add_permanent(ardyn_perm);

        // Add a Demon that entered last turn (no summoning sickness)
        let demon = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec!["Vampire".to_string(), "Demon".to_string()],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        let demon_perm = crate::game::zones::Permanent::new(demon, 1);
        state.battlefield.add_permanent(demon_perm);

        let initial_life = state.life;
        let damage = simulate_combat(&mut state, false);

        // Demon (6) + Ardyn (4) = 10 damage
        assert_eq!(damage, 10);
        // Demon dealt 6 damage with lifelink
        assert_eq!(state.life, initial_life + 6);
    }

    #[test]
    fn test_starscourge_trigger() {
        let mut state = GameState::new();
        state.turn = 2;

        // Add Ardyn to battlefield
        let ardyn = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Ardyn, the Usurper".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 4,
            toughness: 4,
            is_legendary: true,
            creature_types: vec!["Elder".to_string(), "Human".to_string(), "Noble".to_string()],
            abilities: vec!["starscourge".to_string()],
            impending_cost: None,
            impending_counters: None,
        });

        let ardyn_perm = crate::game::zones::Permanent::new(ardyn, 1);
        state.battlefield.add_permanent(ardyn_perm);

        // Add Bringer to graveyard
        let bringer = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Bringer of the Last Gift".to_string(),
                mana_cost: Default::default(),
                mana_value: 8,
            },
            power: 6,
            toughness: 6,
            is_legendary: false,
            creature_types: vec!["Vampire".to_string(), "Demon".to_string()],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        state.graveyard.add_card(bringer);

        // Simulate combat - Starscourge should trigger
        let damage = simulate_combat(&mut state, false);

        // Bringer should be exiled from graveyard
        assert!(state.graveyard.cards().iter().all(|c| c.name() != "Bringer of the Last Gift"));

        // A 5/5 Demon token should be created
        let token_count = state.battlefield.permanents().iter()
            .filter(|p| p.is_copy_of.as_deref() == Some("Bringer of the Last Gift"))
            .count();
        assert_eq!(token_count, 1);

        // Token should have attacked (has haste from Ardyn)
        // Ardyn (4) + Starscourge token (5) = 9 damage
        assert_eq!(damage, 9);
    }
}

