package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.*;
import com.mtg.reanimator.game.*;
import com.mtg.reanimator.game.zones.*;
import com.mtg.reanimator.rng.GameRng;

import java.util.*;

/**
 * Main simulation engine for running MTG Reanimator games.
 * Matches the Rust engine.rs implementation exactly.
 */
public final class SimulationEngine {

    // Known mill enabler cards for mulligan decisions
    private static final Set<String> MILL_ENABLERS = Set.of(
        "Stitcher's Supplier",
        "Teachings of the Kirin",
        "Town Greeter",
        "Overlord of the Balemurk",
        "Kiora, the Rising Tide",
        "Cache Grab",
        "Dredger's Insight",
        "Awaken the Honored Dead"
    );

    // Land-finding spells
    private static final Set<String> LAND_FINDERS = Set.of(
        "Cache Grab",
        "Dredger's Insight",
        "Town Greeter"
    );

    // Known Demons in the game
    private static final Set<String> KNOWN_DEMONS = Set.of(
        "Bringer of the Last Gift"
    );

    private SimulationEngine() {
        // Utility class - prevent instantiation
    }

    // ==================== WIN CONDITION ====================

    /**
     * Check if the game has been won.
     */
    public static boolean checkWinCondition(GameState state) {
        return state.getOpponentLife() <= 0;
    }

    // ==================== COLOR AVAILABILITY ====================

    /**
     * Get available mana colors from battlefield lands.
     * Uses canTapForMana to correctly handle conditional lands.
     */
    public static ColorFlags getAvailableColors(GameState state) {
        ColorFlags colors = new ColorFlags();
        Battlefield battlefield = state.getBattlefield();

        for (Permanent permanent : battlefield.getPermanents()) {
            if (permanent.isLand()) {
                ColorFlags landColors = ManaUtils.getProducedColors(
                    permanent, battlefield, null, state.getLife()
                );
                // Merge colors using bitwise OR
                colors = new ColorFlags(colors.getFlags() | landColors.getFlags());
            }
        }

        return colors;
    }

    // ==================== ARDYN AND DEMON CHECKS ====================

    /**
     * Check if Ardyn, the Usurper is on the battlefield.
     */
    public static boolean hasArdynOnBattlefield(GameState state) {
        return state.getBattlefield().getPermanents().stream()
            .anyMatch(p -> p.getName().equals("Ardyn, the Usurper")
                || "Ardyn, the Usurper".equals(p.getIsCopyOf()));
    }

    /**
     * Check if a permanent is a Demon.
     */
    public static boolean isDemon(Permanent permanent) {
        Card card = permanent.getCard();

        // Check if the card itself is a Demon
        boolean cardIsDemon = false;
        if (card instanceof Card.Creature creature) {
            cardIsDemon = creature.getCreatureTypes().contains("Demon");
        }

        // Check if this is a copy of a known Demon
        String copyOf = permanent.getIsCopyOf();
        boolean copyIsDemon = copyOf != null && KNOWN_DEMONS.contains(copyOf);

        return cardIsDemon || copyIsDemon;
    }

    // ==================== MULLIGAN LOGIC ====================

    /**
     * Count lands in a list of cards.
     */
    private static int countLands(List<Card> cards) {
        return (int) cards.stream().filter(c -> c instanceof Card.Land).count();
    }

    /**
     * Check if a card is a mill/surveil enabler.
     */
    private static boolean isMillEnabler(Card card) {
        return MILL_ENABLERS.contains(card.getName());
    }

    /**
     * Check if a card is a playable early spell.
     */
    private static boolean isPlayableEarlySpell(Card card) {
        return card.getManaValue() <= 3 && !(card instanceof Card.Land);
    }

    /**
     * Decide whether to mulligan a hand.
     */
    public static boolean shouldMulligan(List<Card> hand, int mulliganCount) {
        int lands = countLands(hand);

        // At 4 cards or fewer, keep almost anything with 2+ lands
        if (hand.size() <= 4) {
            return lands < 2;
        }

        // Check for mill enablers - always keep if we have one
        if (hand.stream().anyMatch(SimulationEngine::isMillEnabler)) {
            return lands < 2;
        }

        // Check for playable early spells
        boolean hasEarlySpell = hand.stream().anyMatch(SimulationEngine::isPlayableEarlySpell);

        // Keep if we have 2-5 lands and at least one early spell
        if (lands >= 2 && lands <= 5 && hasEarlySpell) {
            return false;
        }

        // Mulligan if we don't have enough lands or playable spells
        return lands < 2 || !hasEarlySpell;
    }
}
