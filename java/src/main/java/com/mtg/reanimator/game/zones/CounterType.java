package com.mtg.reanimator.game.zones;

/**
 * Counter types for permanents.
 * Matches the Rust CounterType enum.
 */
public enum CounterType {
    /**
     * Time counters - used for impending creatures and similar effects.
     */
    TIME,

    /**
     * Lore counters - used for Sagas.
     */
    LORE
}

