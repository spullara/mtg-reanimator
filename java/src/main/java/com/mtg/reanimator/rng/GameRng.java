package com.mtg.reanimator.rng;

import java.security.SecureRandom;
import java.util.Collections;
import java.util.List;

/**
 * Seeded random number generator for reproducible simulations.
 * Uses Mulberry32 PRNG - MUST match the Rust/TypeScript implementation exactly
 * to allow running identical games across different language implementations.
 */
public class GameRng {
    private long state;

    /**
     * Create a new GameRng with the specified seed.
     * Uses lower 32 bits of the seed to match Rust/TypeScript behavior.
     */
    public GameRng(long seed) {
        this.state = seed & 0xFFFFFFFFL;
    }

    /**
     * Create a new GameRng with a random seed from SecureRandom.
     */
    public GameRng() {
        this(new SecureRandom().nextLong());
    }

    /**
     * Generate next random number in [0, 1).
     * Mulberry32 algorithm - matches TypeScript and Rust exactly.
     */
    public double next() {
        // state = state.wrapping_add(0x6D2B79F5)
        state = (state + 0x6D2B79F5L) & 0xFFFFFFFFL;
        long t = state;

        // t = (t ^ (t >> 15)).wrapping_mul(t | 1)
        t = ((t ^ (t >>> 15)) * (t | 1)) & 0xFFFFFFFFL;

        // t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61))
        t = (t ^ (t + ((t ^ (t >>> 7)) * (t | 61)) & 0xFFFFFFFFL)) & 0xFFFFFFFFL;

        // result = t ^ (t >> 14)
        long result = (t ^ (t >>> 14)) & 0xFFFFFFFFL;

        return result / 4294967296.0;
    }

    /**
     * Generate a random integer in range [0, bound).
     */
    public int nextInt(int bound) {
        return (int) (next() * bound);
    }

    /**
     * Generate a random integer in range [0, max).
     * Alias for nextInt for API compatibility with Rust version.
     */
    public int randomRange(int max) {
        return (int) Math.floor(next() * max);
    }

    /**
     * Fisher-Yates shuffle for a list.
     * Matches TypeScript/Rust shuffle exactly.
     */
    public <T> void shuffle(List<T> list) {
        for (int i = list.size() - 1; i >= 1; i--) {
            int j = (int) Math.floor(next() * (i + 1));
            Collections.swap(list, i, j);
        }
    }

    /**
     * Fisher-Yates shuffle for an array.
     */
    public void shuffle(int[] array) {
        for (int i = array.length - 1; i >= 1; i--) {
            int j = (int) Math.floor(next() * (i + 1));
            int temp = array[i];
            array[i] = array[j];
            array[j] = temp;
        }
    }

    /**
     * Get the current state (for debugging/testing).
     */
    public long getState() {
        return state;
    }
}

