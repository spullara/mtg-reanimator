package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.*;
import com.mtg.reanimator.game.*;
import com.mtg.reanimator.game.zones.*;
import com.mtg.reanimator.rng.GameRng;

import java.util.*;

/**
 * Turn 4 Combo Failure Analysis.
 * Analyzes why the combo couldn't execute on turn 4 across many simulations.
 * Matches the Rust analyze.rs implementation.
 */
public final class GameAnalyzer {

    private GameAnalyzer() {
        // Utility class - prevent instantiation
    }

    // ==================== FAILURE REASONS ====================

    /**
     * Reasons why the combo couldn't execute on turn 4.
     * Listed in priority order for analysis.
     */
    public enum FailureReason {
        // Primary blockers (mutually exclusive - pick the first that applies)
        INSUFFICIENT_LANDS("Insufficient lands (<4)"),
        MISSING_BLUE("Missing blue mana"),
        MISSING_BLACK("Missing black mana"),
        MISSING_GREEN("Missing green mana"),
        SPIDER_MAN_NOT_IN_HAND("Spider-Man not in hand"),
        NO_BRINGER_IN_GRAVEYARD("No Bringer in graveyard"),
        NO_TERROR_IN_GRAVEYARD("No Terror in graveyard"),
        INSUFFICIENT_DAMAGE("Insufficient damage (<20)"),
        
        // Success case
        COMBO_AVAILABLE("✓ Combo available");

        private final String description;

        FailureReason(String description) {
            this.description = description;
        }

        public String getDescription() {
            return description;
        }

        @Override
        public String toString() {
            return description;
        }
    }

    // ==================== CARD LOCATION TRACKING ====================

    /**
     * Location counts for a specific card across zones.
     */
    public record CardLocation(int inHand, int inGraveyard, int onBattlefield) {
        public static CardLocation empty() {
            return new CardLocation(0, 0, 0);
        }

        public CardLocation addHand(int count) {
            return new CardLocation(inHand + count, inGraveyard, onBattlefield);
        }

        public CardLocation addGraveyard(int count) {
            return new CardLocation(inHand, inGraveyard + count, onBattlefield);
        }

        public CardLocation addBattlefield(int count) {
            return new CardLocation(inHand, inGraveyard, onBattlefield + count);
        }
    }

    /**
     * Locations of key combo pieces.
     */
    public record CardLocations(
        CardLocation spiderMan,
        CardLocation bringer,
        CardLocation terror
    ) {
        public static CardLocations empty() {
            return new CardLocations(
                CardLocation.empty(),
                CardLocation.empty(),
                CardLocation.empty()
            );
        }
    }

    // ==================== ANALYSIS RESULTS ====================

    /**
     * Results from analyzing a single game at turn 4.
     */
    public record Turn4Analysis(
        FailureReason primaryFailure,
        int landsCount,
        boolean hasBlue,
        boolean hasBlack,
        boolean hasGreen
    ) {}

    /**
     * Aggregate results from analyzing many games.
     */
    public record AnalysisResults(
        Map<FailureReason, Integer> failureCounts,
        double avgLands,
        double blueAvailability,
        double blackAvailability,
        double greenAvailability
    ) {}

    // ==================== STATE ANALYSIS ====================

    /**
     * Analyze the game state at turn 4 to determine why combo couldn't fire.
     * Should be called at the START of turn 4's main phase (after draw).
     */
    public static Turn4Analysis analyzeTurn4State(GameState state) {
        // Count lands on battlefield
        int landsOnBattlefield = (int) state.getBattlefield().getPermanents().stream()
            .filter(Permanent::isLand)
            .count();

        // Check available colors from lands currently on battlefield
        boolean hasBlue = false;
        boolean hasBlack = false;
        boolean hasGreen = false;

        Battlefield battlefield = state.getBattlefield();
        for (Permanent permanent : battlefield.getPermanents()) {
            if (permanent.isLand()) {
                ColorFlags colors = ManaUtils.getProducedColors(
                    permanent, battlefield, null, state.getLife()
                );
                if (colors.hasBlue()) hasBlue = true;
                if (colors.hasBlack()) hasBlack = true;
                if (colors.hasGreen()) hasGreen = true;
            }
        }

        // Check if we have a land in hand that enters untapped on turn 4
        boolean landInHandUntapped = false;
        boolean landHasBlue = false;
        boolean landHasBlack = false;
        boolean landHasGreen = false;

        for (Card card : state.getHand().getCards()) {
            if (card instanceof Card.Land land) {
                boolean entersTapped = checkLandEntersTapped(land, landsOnBattlefield, state.getTurn(), state.getLife());

                if (!entersTapped) {
                    landInHandUntapped = true;
                    // Check what colors this land provides
                    for (ManaColor color : land.getColors()) {
                        switch (color) {
                            case BLUE -> landHasBlue = true;
                            case BLACK -> landHasBlack = true;
                            case GREEN -> landHasGreen = true;
                            default -> {}
                        }
                    }
                }
            }
        }

        // Total available mana = lands on battlefield + (1 if untapped land in hand)
        int totalMana = landsOnBattlefield + (landInHandUntapped ? 1 : 0);

        // Colors available = battlefield colors + hand land colors (if untapped)
        if (landInHandUntapped) {
            hasBlue = hasBlue || landHasBlue;
            hasBlack = hasBlack || landHasBlack;
            hasGreen = hasGreen || landHasGreen;
        }

        // Find card locations
        CardLocations locations = findCardLocations(state);

        // Calculate expected damage
        int comboDamage = CardResolver.calculateComboDamage(state);

        // Determine primary failure reason (in priority order)
        FailureReason primaryFailure = determinePrimaryFailure(
            totalMana, hasBlue, hasBlack, hasGreen,
            locations, comboDamage, state.getOpponentLife()
        );

        return new Turn4Analysis(primaryFailure, totalMana, hasBlue, hasBlack, hasGreen);
    }

    /**
     * Check if a land would enter tapped on a given turn.
     */
    private static boolean checkLandEntersTapped(Card.Land land, int landsOnBattlefield, int turn, int life) {
        LandSubtype subtype = land.getSubtype();

        return switch (subtype) {
            case FASTLAND -> landsOnBattlefield >= 3; // Tapped if 3+ other lands
            case TOWN -> turn > 3; // Starting Town tapped on turn 4+
            case SHOCK -> life <= 2; // Can pay 2 life to enter untapped
            case UTILITY -> {
                // Verge lands enter untapped
                if (land.getName().endsWith("Verge")) {
                    yield false;
                }
                yield land.isEntersTapped();
            }
            default -> land.isEntersTapped();
        };
    }

    /**
     * Find locations of key combo pieces across all zones.
     */
    private static CardLocations findCardLocations(GameState state) {
        CardLocation spiderMan = CardLocation.empty();
        CardLocation bringer = CardLocation.empty();
        CardLocation terror = CardLocation.empty();

        // Check hand
        for (Card card : state.getHand().getCards()) {
            switch (card.getName()) {
                case "Superior Spider-Man" -> spiderMan = spiderMan.addHand(1);
                case "Bringer of the Last Gift" -> bringer = bringer.addHand(1);
                case "Terror of the Peaks" -> terror = terror.addHand(1);
            }
        }

        // Check graveyard
        for (Card card : state.getGraveyard().getCards()) {
            switch (card.getName()) {
                case "Superior Spider-Man" -> spiderMan = spiderMan.addGraveyard(1);
                case "Bringer of the Last Gift" -> bringer = bringer.addGraveyard(1);
                case "Terror of the Peaks" -> terror = terror.addGraveyard(1);
            }
        }

        // Check battlefield
        for (Permanent perm : state.getBattlefield().getPermanents()) {
            switch (perm.getName()) {
                case "Superior Spider-Man" -> spiderMan = spiderMan.addBattlefield(1);
                case "Bringer of the Last Gift" -> bringer = bringer.addBattlefield(1);
                case "Terror of the Peaks" -> terror = terror.addBattlefield(1);
            }
        }

        return new CardLocations(spiderMan, bringer, terror);
    }

    /**
     * Determine the primary failure reason based on game state.
     * Checks in priority order and returns the first failure found.
     */
    private static FailureReason determinePrimaryFailure(
            int landsCount,
            boolean hasBlue,
            boolean hasBlack,
            boolean hasGreen,
            CardLocations locations,
            int comboDamage,
            int opponentLife) {

        // 1. Not enough lands
        if (landsCount < 4) {
            return FailureReason.INSUFFICIENT_LANDS;
        }

        // 2. Missing colors (Spider-Man costs 1UBG)
        if (!hasBlue) {
            return FailureReason.MISSING_BLUE;
        }
        if (!hasBlack) {
            return FailureReason.MISSING_BLACK;
        }
        if (!hasGreen) {
            return FailureReason.MISSING_GREEN;
        }

        // 3. Spider-Man not in hand
        if (locations.spiderMan().inHand() == 0) {
            return FailureReason.SPIDER_MAN_NOT_IN_HAND;
        }

        // 4. No Bringer in graveyard to copy
        if (locations.bringer().inGraveyard() == 0) {
            return FailureReason.NO_BRINGER_IN_GRAVEYARD;
        }

        // 5. No Terror in graveyard for damage (battlefield also works)
        boolean hasTerrorSource = locations.terror().inGraveyard() > 0
            || locations.terror().onBattlefield() > 0;
        if (!hasTerrorSource) {
            return FailureReason.NO_TERROR_IN_GRAVEYARD;
        }

        // 6. Not enough damage
        if (comboDamage < opponentLife) {
            return FailureReason.INSUFFICIENT_DAMAGE;
        }

        // All requirements met!
        return FailureReason.COMBO_AVAILABLE;
    }

    // ==================== GAME SIMULATION ====================

    /**
     * Run a game to turn 4 only (for analysis).
     * Analyzes state at the START of turn 4 (after draw, before main phase).
     *
     * @param deck The deck to simulate
     * @param seed Random seed for reproducibility
     * @param db Card database
     * @return Analysis of the turn 4 game state
     */
    public static Turn4Analysis runGameToTurn4(List<Card> deck, long seed, CardDatabase db) {
        GameRng rng = new GameRng(seed);

        // Initialize game state
        GameState state = new GameState();

        // Determine if on play or draw (50/50) - BEFORE shuffling to match RNG sequence
        state.setOnThePlay(rng.next() < 0.5);

        // Shuffle deck into library
        List<Card> shuffledDeck = new ArrayList<>(deck);
        rng.shuffle(shuffledDeck);
        for (Card card : shuffledDeck) {
            state.getLibrary().addCard(card);
        }

        // Mulligan phase: resolve mulligans to get opening hand
        List<Card> libraryCards = new ArrayList<>();
        while (state.getLibrary().size() > 0) {
            Card card = state.getLibrary().draw();
            if (card != null) {
                libraryCards.add(card);
            }
        }

        List<Card> openingHand = SimulationEngine.resolveMulligans(libraryCards, rng);

        // Put remaining cards back in library
        for (Card card : libraryCards) {
            state.getLibrary().addCard(card);
        }

        // Add opening hand to hand
        for (Card card : openingHand) {
            state.getHand().add(card);
        }

        // Run turns 1-3 fully
        for (int i = 0; i < 3; i++) {
            SimulationEngine.executeTurn(state, db, false, rng);
        }

        // Turn 4: only do start_turn, upkeep, draw, and precombat main start - then analyze
        TurnManager.startTurn(state);
        TurnManager.upkeepPhase(state);
        TurnManager.drawPhase(state);
        TurnManager.precombatMainPhaseStart(state, false);

        // Analyze state at START of turn 4 main phase
        return analyzeTurn4State(state);
    }

    // ==================== RESULT AGGREGATION ====================

    /**
     * Aggregate results from multiple analyses.
     *
     * @param analyses List of individual turn 4 analyses
     * @return Aggregated results with failure counts and averages
     */
    public static AnalysisResults aggregateResults(List<Turn4Analysis> analyses) {
        if (analyses.isEmpty()) {
            return new AnalysisResults(
                new HashMap<>(),
                0.0,
                0.0,
                0.0,
                0.0
            );
        }

        Map<FailureReason, Integer> failureCounts = new EnumMap<>(FailureReason.class);
        long totalLands = 0;
        int blueCount = 0;
        int blackCount = 0;
        int greenCount = 0;

        for (Turn4Analysis analysis : analyses) {
            failureCounts.merge(analysis.primaryFailure(), 1, Integer::sum);
            totalLands += analysis.landsCount();
            if (analysis.hasBlue()) blueCount++;
            if (analysis.hasBlack()) blackCount++;
            if (analysis.hasGreen()) greenCount++;
        }

        double n = analyses.size();
        return new AnalysisResults(
            failureCounts,
            totalLands / n,
            blueCount / n * 100.0,
            blackCount / n * 100.0,
            greenCount / n * 100.0
        );
    }

    /**
     * Format analysis results as a human-readable string.
     *
     * @param results The aggregated results to format
     * @param totalGames Total number of games analyzed
     * @return Formatted string with failure breakdown
     */
    public static String formatResults(AnalysisResults results, int totalGames) {
        StringBuilder sb = new StringBuilder();
        sb.append("=== Turn 4 Analysis Results ===\n");
        sb.append(String.format("Total games analyzed: %d%n", totalGames));
        sb.append(String.format("Average lands at turn 4: %.2f%n", results.avgLands()));
        sb.append(String.format("Color availability: U=%.1f%% B=%.1f%% G=%.1f%%%n",
            results.blueAvailability(),
            results.blackAvailability(),
            results.greenAvailability()));
        sb.append("\nFailure breakdown:\n");

        // Sort by count descending
        List<Map.Entry<FailureReason, Integer>> sorted = results.failureCounts().entrySet().stream()
            .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
            .toList();

        for (Map.Entry<FailureReason, Integer> entry : sorted) {
            double pct = (double) entry.getValue() / totalGames * 100.0;
            sb.append(String.format("  %s: %d (%.1f%%)%n",
                entry.getKey().getDescription(),
                entry.getValue(),
                pct));
        }

        return sb.toString();
    }
}

