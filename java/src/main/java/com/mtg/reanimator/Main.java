package com.mtg.reanimator;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardDatabase;
import com.mtg.reanimator.card.CardDatabaseException;
import com.mtg.reanimator.simulation.Deck;
import com.mtg.reanimator.simulation.GameAnalyzer;
import com.mtg.reanimator.simulation.GameResult;
import com.mtg.reanimator.simulation.LandOptimizer;
import com.mtg.reanimator.simulation.SimulationEngine;
import picocli.CommandLine;
import picocli.CommandLine.*;

import java.util.*;
import java.util.concurrent.Callable;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.stream.IntStream;

/**
 * MTG Reanimator CLI - Main entry point.
 * Matches the Rust main.rs implementation.
 */
@Command(name = "mtg-reanimator",
        mixinStandardHelpOptions = true,
        version = "1.0",
        description = "MTG Reanimator combo simulator",
        subcommands = {
                Main.RunCommand.class,
                Main.CompareCommand.class,
                Main.OptimizeCommand.class,
                Main.AnalyzeCommand.class
        })
public class Main implements Runnable {

    public static void main(String[] args) {
        int exitCode = new CommandLine(new Main()).execute(args);
        System.exit(exitCode);
    }

    @Override
    public void run() {
        // Show help if no subcommand
        CommandLine.usage(this, System.out);
    }

    // ========== RUN COMMAND ==========
    @Command(name = "run", description = "Run simulations")
    static class RunCommand implements Callable<Integer> {
        @Option(names = {"-n", "--num-games"}, defaultValue = "1000",
                description = "Number of games to simulate")
        int numGames;

        @Option(names = {"-s", "--seed"},
                description = "Random seed (optional)")
        Long seed;

        @Option(names = {"-v", "--verbose"},
                description = "Verbose output (single game trace)")
        boolean verbose;

        @Option(names = {"-d", "--deck"}, defaultValue = "deck.txt",
                description = "Path to deck file")
        String deckPath;

        @Option(names = {"-c", "--cards"}, defaultValue = "cards.json",
                description = "Path to cards database")
        String cardsPath;

        @Override
        public Integer call() throws Exception {
            // Load card database
            CardDatabase db;
            try {
                db = CardDatabase.fromFile(cardsPath);
                System.err.println("✓ Loaded " + db.cardCount() + " cards from " + cardsPath);
            } catch (CardDatabaseException e) {
                System.err.println("✗ Failed to load cards: " + e.getMessage());
                return 1;
            }

            // Load deck
            Deck deck;
            try {
                deck = Deck.loadFromFile(deckPath, db);
            } catch (Deck.DeckException e) {
                System.err.println("✗ Failed to parse deck file '" + deckPath + "': " + e.getMessage());
                return 1;
            }

            System.out.println("\n=== MTG Reanimator Simulator ===\n");
            System.out.println("Deck: " + deckPath + " (" + deck.size() + " cards)");
            System.out.println("Games: " + numGames);
            if (seed != null) {
                System.out.println("Seed: " + seed);
            }
            System.out.println();

            // Run simulations
            long startTime = System.currentTimeMillis();
            List<GameResult> results = runSimulations(deck.getCards(), db, numGames, seed, verbose);
            long elapsed = System.currentTimeMillis() - startTime;

            // Calculate statistics
            printResults(results, numGames, elapsed);
            return 0;
        }
    }

    // ========== COMPARE COMMAND ==========
    @Command(name = "compare", description = "Compare two deck configurations")
    static class CompareCommand implements Callable<Integer> {
        @Parameters(index = "0", description = "First deck file")
        String deck1Path;

        @Parameters(index = "1", description = "Second deck file")
        String deck2Path;

        @Option(names = {"-n", "--num-games"}, defaultValue = "1000",
                description = "Number of games per deck")
        int numGames;

        @Option(names = {"-c", "--cards"}, defaultValue = "cards.json",
                description = "Path to cards database")
        String cardsPath;

        @Override
        public Integer call() throws Exception {
            CardDatabase db;
            try {
                db = CardDatabase.fromFile(cardsPath);
                System.err.println("✓ Loaded " + db.cardCount() + " cards from " + cardsPath);
            } catch (CardDatabaseException e) {
                System.err.println("✗ Failed to load cards: " + e.getMessage());
                return 1;
            }

            Deck deck1, deck2;
            try {
                deck1 = Deck.loadFromFile(deck1Path, db);
            } catch (Deck.DeckException e) {
                System.err.println("✗ Failed to parse deck1 '" + deck1Path + "': " + e.getMessage());
                return 1;
            }

            try {
                deck2 = Deck.loadFromFile(deck2Path, db);
            } catch (Deck.DeckException e) {
                System.err.println("✗ Failed to parse deck2 '" + deck2Path + "': " + e.getMessage());
                return 1;
            }

            System.out.println("\n=== MTG Deck Comparison ===\n");
            System.out.println("Deck 1: " + deck1Path);
            System.out.println("Deck 2: " + deck2Path);
            System.out.println("Games per deck: " + numGames);
            System.out.println();

            long startTime = System.currentTimeMillis();

            System.out.println("Running deck 1...");
            List<GameResult> results1 = runSimulations(deck1.getCards(), db, numGames, null, false);

            System.out.println("Running deck 2...");
            List<GameResult> results2 = runSimulations(deck2.getCards(), db, numGames, null, false);

            long elapsed = System.currentTimeMillis() - startTime;

            printComparisonResults(deck1Path, deck2Path, results1, results2, numGames, elapsed);
            return 0;
        }
    }

    // ========== OPTIMIZE COMMAND ==========
    @Command(name = "optimize", description = "Optimize land configuration")
    static class OptimizeCommand implements Callable<Integer> {
        @Option(names = {"-c", "--configs"}, defaultValue = "100",
                description = "Number of random configurations to test")
        int configs;

        @Option(names = {"-g", "--games"}, defaultValue = "1000",
                description = "Number of games per configuration")
        int games;

        @Option(names = {"-s", "--strategy"}, defaultValue = "weighted",
                description = "Strategy: weighted or shuffle")
        String strategy;

        @Option(names = {"-d", "--deck"}, defaultValue = "deck.txt",
                description = "Base deck file (lands will be replaced)")
        String deckPath;

        @Option(names = {"--cards"}, defaultValue = "cards.json",
                description = "Path to cards database")
        String cardsPath;

        @Override
        public Integer call() throws Exception {
            // Load card database
            CardDatabase db;
            try {
                db = CardDatabase.fromFile(cardsPath);
                System.err.println("✓ Loaded " + db.cardCount() + " cards from " + cardsPath);
            } catch (CardDatabaseException e) {
                System.err.println("✗ Failed to load cards: " + e.getMessage());
                return 1;
            }

            // Validate strategy
            String strategyDesc;
            switch (strategy) {
                case "weighted" -> strategyDesc = "Random counts for each land type, respecting max limits";
                case "shuffle" -> strategyDesc = "Pool of max copies shuffled, take first 24";
                default -> {
                    System.err.println("Unknown strategy '" + strategy + "'. Use 'weighted' or 'shuffle'.");
                    return 1;
                }
            }

            // Extract fixed cards from deck
            LandOptimizer.FixedCards fixedCards;
            try {
                fixedCards = LandOptimizer.extractFixedCardsFromDeck(deckPath, db);
            } catch (Exception e) {
                System.err.println("✗ Failed to extract fixed cards from '" + deckPath + "': " + e.getMessage());
                return 1;
            }

            int fixedCardCount = fixedCards.totalCount();

            System.out.println("\n=== MTG Land Optimization ===\n");
            System.out.println("Base deck: " + deckPath);
            System.out.println("Strategy: " + strategy);
            System.out.println("  - " + strategyDesc + "\n");
            System.out.println("Testing " + configs + " random land configurations");
            System.out.println("Running " + games + " games per configuration...\n");
            System.out.println("Fixed non-land cards: " + fixedCardCount + " cards");
            System.out.println("Land slots to fill: 24 cards\n");

            // Track optimization state
            Map<String, Integer> bestConfig = null;
            double bestAvgTurn = Double.POSITIVE_INFINITY;
            double bestWinRate = 0.0;
            Map<Integer, Integer> bestTurnDistribution = new HashMap<>();
            List<ConfigResult> allResults = new ArrayList<>();

            long startTime = System.currentTimeMillis();

            for (int i = 0; i < configs; i++) {
                // Generate random land configuration
                var rng = new com.mtg.reanimator.rng.GameRng(System.nanoTime() + i);
                Map<String, Integer> config = strategy.equals("shuffle")
                        ? LandOptimizer.generateRandomLandConfigShuffle(rng)
                        : LandOptimizer.generateRandomLandConfigWeighted(rng);

                // Build deck from config
                List<Card> deck;
                try {
                    deck = LandOptimizer.buildDeckFromConfig(config, fixedCards, db);
                } catch (Exception e) {
                    System.err.println("Failed to build deck: " + e.getMessage());
                    continue;
                }

                // Run games using virtual threads for parallelism
                List<GameResult> results = runGamesParallel(deck, db, games);

                // Calculate statistics
                List<GameResult> wins = results.stream().filter(GameResult::isWin).toList();
                double winRate = (double) wins.size() / games;
                double avgWinTurn = wins.isEmpty() ? 0.0 :
                        wins.stream().mapToInt(GameResult::winTurn).average().orElse(0.0);

                allResults.add(new ConfigResult(new HashMap<>(config), winRate, avgWinTurn));

                // Track best configuration (by average win turn)
                if (avgWinTurn > 0.0 && avgWinTurn < bestAvgTurn) {
                    bestConfig = new HashMap<>(config);
                    bestAvgTurn = avgWinTurn;
                    bestWinRate = winRate;

                    // Build turn distribution
                    bestTurnDistribution.clear();
                    for (GameResult result : wins) {
                        if (result.winTurn() != null) {
                            bestTurnDistribution.merge(result.winTurn(), 1, Integer::sum);
                        }
                    }

                    System.out.printf("[%d/%d] New best! Avg turn: %.3f, Win rate: %.1f%%%n",
                            i + 1, configs, bestAvgTurn, bestWinRate * 100.0);
                    System.out.println("  Lands: " + LandOptimizer.configToString(config) + "\n");
                }

                // Progress update every 100 configs (or 10 if testing small)
                int progressInterval = configs >= 100 ? 100 : 10;
                if ((i + 1) % progressInterval == 0) {
                    long elapsed = System.currentTimeMillis() - startTime;
                    double elapsedSec = elapsed / 1000.0;
                    double eta = (elapsedSec / (i + 1)) * (configs - i - 1);
                    System.out.printf("Progress: %d/%d (%.1f%%) - ETA: %.0fs%n",
                            i + 1, configs, (i + 1) * 100.0 / configs, eta);
                }
            }

            long totalTime = System.currentTimeMillis() - startTime;

            System.out.println("\n=== Optimization Complete ===");
            System.out.printf("Total time: %.1fs%n", totalTime / 1000.0);
            System.out.println("Configurations tested: " + configs);
            System.out.println("Games per config: " + games);
            System.out.println("Total games: " + (configs * games) + "\n");

            System.out.println("=== BEST LAND CONFIGURATION ===");
            System.out.printf("Average win turn: %.3f%n", bestAvgTurn);
            System.out.printf("Win rate: %.1f%%%n", bestWinRate * 100.0);
            System.out.println("\nLand breakdown:");
            if (bestConfig != null) {
                bestConfig.entrySet().stream()
                        .filter(e -> e.getValue() > 0)
                        .sorted((a, b) -> {
                            int cmp = Integer.compare(b.getValue(), a.getValue());
                            return cmp != 0 ? cmp : a.getKey().compareTo(b.getKey());
                        })
                        .forEach(e -> System.out.println("  " + e.getValue() + " " + e.getKey()));
            }

            // Show top 10 configurations
            System.out.println("\n=== Top 10 Configurations ===");
            allResults.sort(Comparator.comparingDouble(ConfigResult::avgWinTurn));
            allResults.stream()
                    .filter(r -> r.avgWinTurn() > 0)
                    .limit(10)
                    .forEach(r -> {
                        int idx = allResults.indexOf(r) + 1;
                        System.out.printf("[%d] Avg turn: %.3f, Win rate: %.1f%%%n",
                                idx, r.avgWinTurn(), r.winRate() * 100.0);
                        System.out.println("    " + LandOptimizer.configToString(r.config()));
                    });

            // Save best deck to file
            if (bestConfig != null) {
                try {
                    var saveParams = new LandOptimizer.DeckSaveParams(
                            bestWinRate, bestAvgTurn, games, strategy,
                            bestTurnDistribution, fixedCards);
                    String filename = LandOptimizer.saveDeckToFile(bestConfig, saveParams);
                    System.out.println("\n✓ Best deck saved to: " + filename);
                } catch (Exception e) {
                    System.err.println("Failed to save deck: " + e.getMessage());
                }
            }

            return 0;
        }
    }

    /**
     * Result of testing a configuration.
     */
    record ConfigResult(Map<String, Integer> config, double winRate, double avgWinTurn) {}

    /**
     * Run games in parallel using virtual threads.
     */
    private static List<GameResult> runGamesParallel(List<Card> deck, CardDatabase db, int numGames) {
        try (var executor = Executors.newVirtualThreadPerTaskExecutor()) {
            List<Future<GameResult>> futures = new ArrayList<>();
            for (int j = 0; j < numGames; j++) {
                final long seed = System.nanoTime() + j;
                futures.add(executor.submit(() ->
                        SimulationEngine.runGame(deck, seed, db, false)));
            }

            List<GameResult> results = new ArrayList<>();
            for (Future<GameResult> future : futures) {
                try {
                    results.add(future.get());
                } catch (Exception e) {
                    // Skip failed games
                }
            }
            return results;
        }
    }

    // ========== ANALYZE COMMAND ==========
    @Command(name = "analyze", description = "Analyze Turn 4 failures")
    static class AnalyzeCommand implements Callable<Integer> {
        @Option(names = {"-n", "--num-games"}, defaultValue = "1000",
                description = "Number of games to simulate")
        int numGames;

        @Option(names = {"-s", "--seed"},
                description = "Random seed (optional)")
        Long seed;

        @Option(names = {"-d", "--deck"}, defaultValue = "deck.txt",
                description = "Path to deck file")
        String deckPath;

        @Option(names = {"-c", "--cards"}, defaultValue = "cards.json",
                description = "Path to cards database")
        String cardsPath;

        @Override
        public Integer call() throws Exception {
            // Load card database
            CardDatabase db;
            try {
                db = CardDatabase.fromFile(cardsPath);
                System.err.println("✓ Loaded " + db.cardCount() + " cards from " + cardsPath);
            } catch (CardDatabaseException e) {
                System.err.println("✗ Failed to load cards: " + e.getMessage());
                return 1;
            }

            // Load deck
            Deck deck;
            try {
                deck = Deck.loadFromFile(deckPath, db);
            } catch (Deck.DeckException e) {
                System.err.println("✗ Failed to parse deck file '" + deckPath + "': " + e.getMessage());
                return 1;
            }

            System.out.println("\n=== Turn 4 Combo Failure Analysis ===\n");
            System.out.println("Deck: " + deckPath + " (" + deck.size() + " cards)");
            System.out.println("Games: " + numGames);
            if (seed != null) {
                System.out.println("Seed: " + seed);
            }
            System.out.println();

            long startTime = System.currentTimeMillis();

            // Run games to turn 4 in parallel
            List<GameAnalyzer.Turn4Analysis> analyses;
            if (seed != null) {
                // Sequential with fixed seed
                analyses = new ArrayList<>();
                for (int i = 0; i < numGames; i++) {
                    analyses.add(GameAnalyzer.runGameToTurn4(deck.getCards(), seed + i, db));
                }
            } else {
                // Parallel with random seeds
                analyses = IntStream.range(0, numGames)
                        .parallel()
                        .mapToObj(i -> {
                            long gameSeed = System.nanoTime() + i;
                            return GameAnalyzer.runGameToTurn4(deck.getCards(), gameSeed, db);
                        })
                        .toList();
            }

            long elapsed = System.currentTimeMillis() - startTime;

            // Aggregate results
            GameAnalyzer.AnalysisResults results = GameAnalyzer.aggregateResults(analyses);

            System.out.println("=== Results ===\n");

            // Sort failures by count (descending)
            List<Map.Entry<GameAnalyzer.FailureReason, Integer>> failures = new ArrayList<>(
                    results.failureCounts().entrySet());
            failures.sort((a, b) -> Integer.compare(b.getValue(), a.getValue()));

            // Print ranked failure reasons
            System.out.println("Failure Reasons (ranked by frequency):\n");
            for (Map.Entry<GameAnalyzer.FailureReason, Integer> entry : failures) {
                double pct = (double) entry.getValue() / numGames * 100.0;
                String bar = "█".repeat((int) (pct / 2.0));
                System.out.printf("  %-30s %5.1f%% %s (%d)%n",
                        entry.getKey().getDescription(), pct, bar, entry.getValue());
            }

            System.out.println("\n--- Statistics ---\n");
            System.out.printf("Average lands by turn 4: %.2f%n", results.avgLands());
            System.out.println("Color availability:");
            System.out.printf("  Blue:  %5.1f%%%n", results.blueAvailability());
            System.out.printf("  Black: %5.1f%%%n", results.blackAvailability());
            System.out.printf("  Green: %5.1f%%%n", results.greenAvailability());

            // Calculate combo ready percentage
            int comboReady = results.failureCounts()
                    .getOrDefault(GameAnalyzer.FailureReason.COMBO_AVAILABLE, 0);
            System.out.printf("%nTurn 4 combo ready: %.1f%% (%d/%d)%n",
                    (double) comboReady / numGames * 100.0, comboReady, numGames);

            double elapsedSec = elapsed / 1000.0;
            double gamesPerSec = elapsedSec > 0 ? numGames / elapsedSec : 0;
            System.out.printf("%nCompleted in %.2fs (%.0f games/sec)%n", elapsedSec, gamesPerSec);

            return 0;
        }
    }

    // ========== HELPER METHODS ==========

    /**
     * Run simulations and return results.
     */
    private static List<GameResult> runSimulations(List<Card> deck, CardDatabase db,
                                                   int count, Long seed, boolean verbose) {
        if (seed != null) {
            // Sequential with fixed seed
            List<GameResult> results = new ArrayList<>();
            for (int i = 0; i < count; i++) {
                boolean verboseThisGame = verbose && i == 0;
                results.add(SimulationEngine.runGame(deck, seed + i, db, verboseThisGame));
            }
            return results;
        } else if (verbose) {
            // Sequential for verbose mode
            long baseSeed = System.nanoTime();
            System.out.println("Seed: " + baseSeed);
            List<GameResult> results = new ArrayList<>();
            for (int i = 0; i < count; i++) {
                boolean verboseThisGame = i == 0;
                results.add(SimulationEngine.runGame(deck, baseSeed + i, db, verboseThisGame));
            }
            return results;
        } else {
            // Parallel with random seeds
            return IntStream.range(0, count)
                    .parallel()
                    .mapToObj(i -> {
                        long gameSeed = System.nanoTime() + i;
                        return SimulationEngine.runGame(deck, gameSeed, db, false);
                    })
                    .toList();
        }
    }

    /**
     * Print simulation results.
     */
    private static void printResults(List<GameResult> results, int numGames, long elapsedMs) {
        List<GameResult> wins = results.stream().filter(GameResult::isWin).toList();
        double winRate = (double) wins.size() / numGames;

        double avgWinTurn = wins.isEmpty() ? 0.0 :
                wins.stream().mapToInt(r -> r.winTurn()).average().orElse(0.0);

        // Turn distribution
        Map<Integer, Long> turnDist = new TreeMap<>();
        for (GameResult r : results) {
            if (r.winTurn() != null) {
                turnDist.merge(r.winTurn(), 1L, Long::sum);
            }
        }

        // Average UBG turn
        List<GameResult> hasUbg = results.stream()
                .filter(r -> r.turnWithUbg() != null)
                .toList();
        double avgUbgTurn = hasUbg.isEmpty() ? 0.0 :
                hasUbg.stream().mapToInt(r -> r.turnWithUbg()).average().orElse(0.0);

        System.out.println("=== Results ===\n");
        System.out.printf("Win rate: %.1f%% (%d/%d)%n", winRate * 100.0, wins.size(), numGames);
        System.out.printf("Average win turn: %.2f%n", avgWinTurn);
        System.out.printf("Average UBG available: turn %.2f%n", avgUbgTurn);
        System.out.println();

        System.out.println("Turn distribution:");
        for (Map.Entry<Integer, Long> entry : turnDist.entrySet()) {
            double pct = (double) entry.getValue() / numGames * 100.0;
            String bar = "█".repeat((int) (pct / 2.0));
            System.out.printf("  Turn %2d: %5.1f%% %s (%d)%n",
                    entry.getKey(), pct, bar, entry.getValue());
        }

        long noWin = results.stream().filter(r -> r.winTurn() == null).count();
        if (noWin > 0) {
            double pct = (double) noWin / numGames * 100.0;
            System.out.printf("  No win: %5.1f%% (%d)%n", pct, noWin);
        }

        System.out.println();
        double elapsedSec = elapsedMs / 1000.0;
        double gamesPerSec = elapsedSec > 0 ? numGames / elapsedSec : 0;
        System.out.printf("Simulation completed in %.2fs (%.0f games/sec)%n", elapsedSec, gamesPerSec);
    }

    /**
     * Print comparison results.
     */
    private static void printComparisonResults(String deck1Name, String deck2Name,
                                               List<GameResult> results1, List<GameResult> results2,
                                               int numGames, long elapsedMs) {
        List<GameResult> wins1 = results1.stream().filter(GameResult::isWin).toList();
        List<GameResult> wins2 = results2.stream().filter(GameResult::isWin).toList();

        double winRate1 = (double) wins1.size() / numGames;
        double winRate2 = (double) wins2.size() / numGames;

        double avgWin1 = wins1.isEmpty() ? 0.0 :
                wins1.stream().mapToInt(r -> r.winTurn()).average().orElse(0.0);
        double avgWin2 = wins2.isEmpty() ? 0.0 :
                wins2.stream().mapToInt(r -> r.winTurn()).average().orElse(0.0);

        System.out.println("\n=== Results ===\n");
        System.out.printf("%-20s %12s %12s%n", "Metric", deck1Name, deck2Name);
        System.out.println("-".repeat(50));
        System.out.printf("%-20s %11.1f%% %11.1f%%%n", "Win rate", winRate1 * 100.0, winRate2 * 100.0);
        System.out.printf("%-20s %12.2f %12.2f%n", "Avg win turn", avgWin1, avgWin2);
        System.out.println();

        if (winRate1 > winRate2) {
            System.out.printf("✓ %s has %.1f%% higher win rate%n",
                    deck1Name, (winRate1 - winRate2) * 100.0);
        } else if (winRate2 > winRate1) {
            System.out.printf("✓ %s has %.1f%% higher win rate%n",
                    deck2Name, (winRate2 - winRate1) * 100.0);
        } else {
            System.out.println("Both decks have the same win rate");
        }

        if (avgWin1 < avgWin2 && avgWin1 > 0.0) {
            System.out.printf("✓ %s wins %.2f turns faster on average%n",
                    deck1Name, avgWin2 - avgWin1);
        } else if (avgWin2 < avgWin1 && avgWin2 > 0.0) {
            System.out.printf("✓ %s wins %.2f turns faster on average%n",
                    deck2Name, avgWin1 - avgWin2);
        }

        double elapsedSec = elapsedMs / 1000.0;
        System.out.printf("%nCompleted in %.2fs%n", elapsedSec);
    }
}