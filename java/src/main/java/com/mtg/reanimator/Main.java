package com.mtg.reanimator;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardDatabase;
import com.mtg.reanimator.card.CardDatabaseException;
import com.mtg.reanimator.simulation.Deck;
import com.mtg.reanimator.simulation.GameResult;
import com.mtg.reanimator.simulation.SimulationEngine;
import picocli.CommandLine;
import picocli.CommandLine.*;

import java.util.*;
import java.util.concurrent.Callable;
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
        @Option(names = {"-n", "--count"}, defaultValue = "10000",
                description = "Number of simulations")
        int count;

        @Option(names = {"-s", "--seed"},
                description = "Random seed (optional)")
        Long seed;

        @Option(names = {"-t", "--threads"}, defaultValue = "1",
                description = "Number of threads")
        int threads;

        @Option(names = {"-v", "--verbose"},
                description = "Verbose output (single game trace)")
        boolean verbose;

        @Option(names = {"--max-turns"}, defaultValue = "10",
                description = "Maximum turns per game")
        int maxTurns;

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
            System.out.println("Games: " + count);
            if (seed != null) {
                System.out.println("Seed: " + seed);
            }
            System.out.println();

            // Run simulations
            long startTime = System.currentTimeMillis();
            List<GameResult> results = runSimulations(deck.getCards(), db, count, seed, verbose);
            long elapsed = System.currentTimeMillis() - startTime;

            // Calculate statistics
            printResults(results, count, elapsed);
            return 0;
        }
    }

    // ========== COMPARE COMMAND ==========
    @Command(name = "compare", description = "Compare two deck configurations")
    static class CompareCommand implements Callable<Integer> {
        @Option(names = {"--deck1"}, required = true,
                description = "First deck file")
        String deck1Path;

        @Option(names = {"--deck2"}, required = true,
                description = "Second deck file")
        String deck2Path;

        @Option(names = {"-n", "--count"}, defaultValue = "10000",
                description = "Number of simulations per deck")
        int count;

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
            System.out.println("Games per deck: " + count);
            System.out.println();

            long startTime = System.currentTimeMillis();

            System.out.println("Running deck 1...");
            List<GameResult> results1 = runSimulations(deck1.getCards(), db, count, null, false);

            System.out.println("Running deck 2...");
            List<GameResult> results2 = runSimulations(deck2.getCards(), db, count, null, false);

            long elapsed = System.currentTimeMillis() - startTime;

            printComparisonResults(deck1Path, deck2Path, results1, results2, count, elapsed);
            return 0;
        }
    }

    // ========== OPTIMIZE COMMAND ==========
    @Command(name = "optimize", description = "Optimize land configuration")
    static class OptimizeCommand implements Callable<Integer> {
        @Option(names = {"-n", "--count"}, defaultValue = "1000",
                description = "Simulations per configuration")
        int count;

        @Option(names = {"--strategy"}, defaultValue = "weighted",
                description = "Strategy: weighted or shuffle")
        String strategy;

        @Option(names = {"-c", "--cards"}, defaultValue = "cards.json",
                description = "Path to cards database")
        String cardsPath;

        @Override
        public Integer call() throws Exception {
            System.out.println("\n=== MTG Land Optimization ===\n");
            System.out.println("Strategy: " + strategy);
            System.out.println("Simulations per config: " + count);
            System.out.println();
            System.out.println("Note: Land optimization not yet implemented in Java port.");
            System.out.println("Use the Rust version for land optimization.");
            return 0;
        }
    }

    // ========== ANALYZE COMMAND ==========
    @Command(name = "analyze", description = "Analyze Turn 4 failures")
    static class AnalyzeCommand implements Callable<Integer> {
        @Option(names = {"-n", "--count"}, defaultValue = "10000",
                description = "Number of simulations")
        int count;

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
            System.out.println("\n=== Turn 4 Combo Failure Analysis ===\n");
            System.out.println("Deck: " + deckPath);
            System.out.println("Games: " + count);
            if (seed != null) {
                System.out.println("Seed: " + seed);
            }
            System.out.println();
            System.out.println("Note: Turn 4 analysis not yet implemented in Java port.");
            System.out.println("Use the Rust version for detailed failure analysis.");
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