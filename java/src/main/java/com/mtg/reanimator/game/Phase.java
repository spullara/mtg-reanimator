package com.mtg.reanimator.game;

/**
 * Game phases in Magic: The Gathering.
 * Matches the Rust Phase enum.
 */
public enum Phase {
    UNTAP,
    UPKEEP,
    DRAW,
    MAIN1,
    COMBAT,
    MAIN2,
    END;

    /**
     * Check if this is a main phase (when sorceries can be cast).
     */
    public boolean isMainPhase() {
        return this == MAIN1 || this == MAIN2;
    }
}

