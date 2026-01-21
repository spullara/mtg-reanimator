package com.mtg.reanimator.simulation;

/**
 * Result of a single game simulation.
 * Matches the Rust GameResult struct exactly.
 */
public record GameResult(
    /**
     * Turn on which the game was won (null if didn't win by turn 20).
     */
    Integer winTurn,
    
    /**
     * First turn we had access to U, B, and G mana.
     */
    Integer turnWithUbg
) {
    /**
     * Check if the game was won.
     */
    public boolean isWin() {
        return winTurn != null;
    }
}

