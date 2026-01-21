package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardDatabase;
import com.mtg.reanimator.card.CardDatabaseException;
import com.mtg.reanimator.rng.GameRng;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Land configuration optimizer for MTG Reanimator.
 * Matches the Rust optimize.rs implementation.
 */
public final class LandOptimizer {

    /** Standard land count for 60-card deck (60 - 36 spells) */
    public static final int TOTAL_LANDS = 24;

    private LandOptimizer() {
        // Utility class - prevent instantiation
    }

    // ==================== LAND CONFIGURATION TYPES ====================

    /**
     * Land type definition with min/max constraints.
     */
    public record LandType(String name, int min, int max) {}

    /**
     * Land configuration: map of land name to count.
     */
    public static Map<String, Integer> newLandConfig() {
        return new HashMap<>();
    }

    /**
     * Fixed cards configuration: non-land cards extracted from deck file.
     */
    public record FixedCards(List<Map.Entry<String, Integer>> cards) {
        public int totalCount() {
            return cards.stream().mapToInt(Map.Entry::getValue).sum();
        }
    }

    // ==================== LAND TYPE CONSTRAINTS ====================

    /**
     * Get all available land types with their constraints.
     * Matches the Rust get_land_types function exactly.
     */
    public static List<LandType> getLandTypes() {
        return List.of(
            new LandType("Forest", 0, 4),
            new LandType("Island", 0, 4),
            new LandType("Swamp", 0, 4),
            new LandType("Watery Grave", 0, 4),
            new LandType("Undercity Sewers", 0, 4),
            new LandType("Underground Mortuary", 0, 4),
            // 4 Cavern of Souls for anti-counterspell protection
            new LandType("Cavern of Souls", 4, 4),
            new LandType("Restless Cottage", 0, 1),
            new LandType("Wastewood Verge", 0, 4),
            new LandType("Gloomlake Verge", 0, 4),
            new LandType("Multiversal Passage", 0, 4),
            new LandType("Blooming Marsh", 0, 4),
            new LandType("Starting Town", 0, 4)
        );
    }

    // ==================== DECK PARSING ====================

    /**
     * Extract non-land cards from a deck file.
     * @param deckFile Path to the deck file
     * @param db Card database for card lookup
     * @return FixedCards containing non-land cards with counts
     */
    public static FixedCards extractFixedCardsFromDeck(String deckFile, CardDatabase db) 
            throws IOException, CardDatabaseException {
        Map<String, Integer> cardCounts = new HashMap<>();

        try (BufferedReader reader = new BufferedReader(new FileReader(deckFile))) {
            String line;
            while ((line = reader.readLine()) != null) {
                line = line.trim();
                
                // Skip empty lines and comments
                if (line.isEmpty() || line.startsWith("#")) {
                    continue;
                }

                // Parse "N Card Name" format
                int spaceIdx = line.indexOf(' ');
                if (spaceIdx <= 0) {
                    continue;
                }

                String countStr = line.substring(0, spaceIdx);
                String cardName = line.substring(spaceIdx + 1).trim();

                int count;
                try {
                    count = Integer.parseInt(countStr);
                } catch (NumberFormatException e) {
                    continue;
                }

                // Get card from database to check if it's a land
                Card card = db.getCard(cardName);
                if (!(card instanceof Card.Land)) {
                    cardCounts.merge(cardName, count, Integer::sum);
                }
            }
        }

        // Convert to sorted list for consistent ordering
        List<Map.Entry<String, Integer>> sortedCards = cardCounts.entrySet().stream()
            .sorted(Map.Entry.comparingByKey())
            .toList();

        return new FixedCards(sortedCards);
    }

    // ==================== RANDOM CONFIGURATION GENERATION ====================

    /**
     * Generate a random land configuration using weighted strategy.
     * First enforces minimum constraints, then randomly distributes remaining slots.
     */
    public static Map<String, Integer> generateRandomLandConfigWeighted(GameRng rng) {
        Map<String, Integer> config = new HashMap<>();
        int remaining = TOTAL_LANDS;
        List<LandType> landTypes = new ArrayList<>(getLandTypes());

        // First pass: enforce minimum constraints
        for (LandType land : landTypes) {
            if (land.min() > 0) {
                config.put(land.name(), land.min());
                remaining -= land.min();
            }
        }

        // Shuffle land types randomly for variety
        rng.shuffle(landTypes);

        // Second pass: assign random counts respecting max limits
        for (LandType land : landTypes) {
            int current = config.getOrDefault(land.name(), 0);
            int maxAdditional = Math.min(land.max() - current, remaining);
            int additional = rng.randomRange(maxAdditional + 1);
            config.merge(land.name(), additional, Integer::sum);
            remaining -= additional;
        }

        // Third pass: distribute remaining slots
        int attempts = 0;
        while (remaining > 0 && attempts < 1000) {
            int idx = rng.randomRange(landTypes.size());
            LandType land = landTypes.get(idx);
            if (config.getOrDefault(land.name(), 0) < land.max()) {
                config.merge(land.name(), 1, Integer::sum);
                remaining--;
            }
            attempts++;
        }

        return config;
    }

    /**
     * Generate a random land configuration using shuffle strategy.
     * Creates a pool of land slots and shuffles to select.
     */
    public static Map<String, Integer> generateRandomLandConfigShuffle(GameRng rng) {
        Map<String, Integer> config = new HashMap<>();
        List<LandType> landTypes = getLandTypes();
        int remaining = TOTAL_LANDS;

        // First: enforce minimum constraints
        for (LandType land : landTypes) {
            if (land.min() > 0) {
                config.put(land.name(), land.min());
                remaining -= land.min();
            }
        }

        // Create pool with remaining capacity for each land (max - min already used)
        List<String> pool = new ArrayList<>();
        for (LandType land : landTypes) {
            int alreadyUsed = config.getOrDefault(land.name(), 0);
            for (int i = 0; i < (land.max() - alreadyUsed); i++) {
                pool.add(land.name());
            }
        }

        // Shuffle the pool
        rng.shuffle(pool);

        // Take from shuffled pool to fill remaining slots
        int toTake = Math.min(remaining, pool.size());
        for (int i = 0; i < toTake; i++) {
            String landName = pool.get(i);
            config.merge(landName, 1, Integer::sum);
        }

        return config;
    }

    // ==================== DECK BUILDING ====================

    /**
     * Build a complete deck from a land configuration and fixed cards.
     * @param config Land configuration (name -> count)
     * @param fixedCards Fixed non-land cards
     * @param db Card database
     * @return Complete deck as a list of cards
     */
    public static List<Card> buildDeckFromConfig(
            Map<String, Integer> config,
            FixedCards fixedCards,
            CardDatabase db) throws CardDatabaseException {
        List<Card> cards = new ArrayList<>();

        // Add fixed cards
        for (Map.Entry<String, Integer> entry : fixedCards.cards()) {
            String cardName = entry.getKey();
            int count = entry.getValue();
            for (int i = 0; i < count; i++) {
                cards.add(db.getCard(cardName));
            }
        }

        // Add lands from config
        for (Map.Entry<String, Integer> entry : config.entrySet()) {
            String landName = entry.getKey();
            int count = entry.getValue();
            for (int i = 0; i < count; i++) {
                cards.add(db.getCard(landName));
            }
        }

        return cards;
    }

    // ==================== FORMATTING ====================

    /**
     * Format a land configuration as a readable string.
     * Sorts by count (descending), then by name.
     */
    public static String configToString(Map<String, Integer> config) {
        return config.entrySet().stream()
            .filter(e -> e.getValue() > 0)
            .sorted((a, b) -> {
                int cmp = Integer.compare(b.getValue(), a.getValue());
                return cmp != 0 ? cmp : a.getKey().compareTo(b.getKey());
            })
            .map(e -> e.getValue() + " " + e.getKey())
            .collect(Collectors.joining(", "));
    }

    /**
     * Calculate a short hash for a deck configuration.
     */
    public static String calculateDeckHash(Map<String, Integer> config, FixedCards fixedCards) {
        // Combine all cards (fixed + lands) and sort
        List<Map.Entry<String, Integer>> allCards = new ArrayList<>(fixedCards.cards());
        for (Map.Entry<String, Integer> entry : config.entrySet()) {
            if (entry.getValue() > 0) {
                allCards.add(entry);
            }
        }
        allCards.sort(Map.Entry.comparingByKey());

        // Hash the sorted card list
        int hash = 0;
        for (Map.Entry<String, Integer> entry : allCards) {
            hash = hash * 31 + entry.getKey().hashCode();
            hash = hash * 31 + entry.getValue().hashCode();
        }

        // Return first 8 hex chars
        return String.format("%08x", hash & 0xFFFFFFFFL);
    }

    // ==================== DECK SAVING ====================

    /**
     * Parameters for saving a deck configuration.
     */
    public record DeckSaveParams(
        double winRate,
        double avgWinTurn,
        int numSimulations,
        String strategy,
        Map<Integer, Integer> turnDistribution,
        FixedCards fixedCards
    ) {}

    /**
     * Save a deck configuration to a file with optimization results.
     * @return The filename of the saved deck
     */
    public static String saveDeckToFile(Map<String, Integer> config, DeckSaveParams params)
            throws IOException {
        String hash = calculateDeckHash(config, params.fixedCards());
        String filename = "deck_" + hash + ".txt";

        try (PrintWriter writer = new PrintWriter(new FileWriter(filename))) {
            // Write header with metadata
            writer.println("# MTG Reanimator Deck");
            writer.println("# Generated: " + LocalDateTime.now()
                .format(DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")));
            writer.println("# Hash: " + hash);
            writer.println("#");

            // Optimization parameters
            writer.println("# Optimization Results");
            writer.println("# Strategy: " + params.strategy());
            writer.println("# Simulations: " + params.numSimulations());
            writer.printf("# Win rate: %.1f%%%n", params.winRate() * 100.0);
            writer.printf("# Average win turn: %.3f%n", params.avgWinTurn());
            writer.println("#");

            // Turn distribution
            writer.println("# Turn Distribution");
            List<Map.Entry<Integer, Integer>> turns = params.turnDistribution().entrySet().stream()
                .sorted(Map.Entry.comparingByKey())
                .toList();
            int totalWins = params.turnDistribution().values().stream().mapToInt(i -> i).sum();
            for (Map.Entry<Integer, Integer> turn : turns) {
                double pct = totalWins > 0 ? (double) turn.getValue() / totalWins * 100.0 : 0.0;
                writer.printf("# Turn %d: %d (%.1f%%)%n", turn.getKey(), turn.getValue(), pct);
            }
            writer.println();

            // Write fixed cards
            int fixedCardCount = params.fixedCards().totalCount();
            writer.println("# Fixed cards (" + fixedCardCount + ")");
            List<Map.Entry<String, Integer>> sortedFixed = params.fixedCards().cards().stream()
                .sorted(Map.Entry.comparingByKey())
                .toList();
            for (Map.Entry<String, Integer> entry : sortedFixed) {
                writer.println(entry.getValue() + " " + entry.getKey());
            }
            writer.println();

            // Write lands sorted by count then name
            writer.println("# Lands (24)");
            List<Map.Entry<String, Integer>> lands = config.entrySet().stream()
                .filter(e -> e.getValue() > 0)
                .sorted((a, b) -> {
                    int cmp = Integer.compare(b.getValue(), a.getValue());
                    return cmp != 0 ? cmp : a.getKey().compareTo(b.getKey());
                })
                .toList();
            for (Map.Entry<String, Integer> entry : lands) {
                writer.println(entry.getValue() + " " + entry.getKey());
            }
        }

        return filename;
    }
}

