package com.mtg.reanimator.rng;

import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for GameRng to verify it matches Rust/TypeScript implementation exactly.
 */
class GameRngTest {

    @Test
    void testSameSeedProducesSameSequence() {
        GameRng rng1 = new GameRng(12345);
        GameRng rng2 = new GameRng(12345);

        for (int i = 0; i < 100; i++) {
            double v1 = rng1.next();
            double v2 = rng2.next();
            assertEquals(v1, v2, "Same seed should produce same random sequence");
        }
    }

    @Test
    void testDifferentSeedsProduceDifferentSequences() {
        GameRng rng1 = new GameRng(12345);
        GameRng rng2 = new GameRng(54321);

        int sameCount = 0;
        for (int i = 0; i < 100; i++) {
            if (Math.abs(rng1.next() - rng2.next()) < 1e-10) {
                sameCount++;
            }
        }
        assertTrue(sameCount < 5, "Different seeds should produce different sequences");
    }

    @Test
    void testShuffleReproducibility() {
        List<Integer> arr1 = new ArrayList<>(List.of(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));
        List<Integer> arr2 = new ArrayList<>(List.of(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));

        GameRng rng1 = new GameRng(42);
        GameRng rng2 = new GameRng(42);

        rng1.shuffle(arr1);
        rng2.shuffle(arr2);

        assertEquals(arr1, arr2, "Same seed should produce same shuffle");
    }

    @Test
    void testRandomRange() {
        GameRng rng = new GameRng(123);
        for (int i = 0; i < 1000; i++) {
            int val = rng.randomRange(10);
            assertTrue(val >= 0 && val < 10, "randomRange should be in [0, max)");
        }
    }

    /**
     * CRITICAL TEST: Verify exact match with TypeScript/Rust mulberry32(12345)
     * These are the exact values produced by TypeScript's mulberry32 and Rust's implementation.
     */
    @Test
    void testMulberry32MatchesTypeScript() {
        double[] expected = {
            0.9797282677609473,
            0.3067522644996643,
            0.484205421525985,
            0.817934412509203,
            0.5094283693470061,
            0.34747186047025025,
            0.07375754183158278,
            0.7663964673411101,
            0.9968264393974096,
            0.8250224851071835
        };

        GameRng rng = new GameRng(12345);
        for (int i = 0; i < expected.length; i++) {
            double actual = rng.next();
            assertEquals(expected[i], actual, 1e-15,
                String.format("Value %d mismatch: expected %f, got %f", i, expected[i], actual));
        }
    }

    @Test
    void testNextInt() {
        GameRng rng = new GameRng(42);
        for (int i = 0; i < 1000; i++) {
            int val = rng.nextInt(100);
            assertTrue(val >= 0 && val < 100, "nextInt should be in [0, bound)");
        }
    }
}

