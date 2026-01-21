package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Mana colors in Magic: The Gathering.
 * Matches the Rust ManaColor enum exactly.
 */
public enum ManaColor {
    WHITE('W', ColorFlags.WHITE),
    BLUE('U', ColorFlags.BLUE),
    BLACK('B', ColorFlags.BLACK),
    RED('R', ColorFlags.RED),
    GREEN('G', ColorFlags.GREEN),
    COLORLESS('C', ColorFlags.COLORLESS);

    private final char symbol;
    private final int flag;

    ManaColor(char symbol, int flag) {
        this.symbol = symbol;
        this.flag = flag;
    }

    /**
     * Get the single character representation (W/U/B/R/G/C).
     */
    @JsonValue
    public char getSymbol() {
        return symbol;
    }

    /**
     * Get the bit flag for this color.
     */
    public int getFlag() {
        return flag;
    }

    /**
     * Parse a ManaColor from a single character.
     * @param c The character (W/U/B/R/G/C, case-insensitive)
     * @return The corresponding ManaColor
     * @throws IllegalArgumentException if the character is not a valid color
     */
    public static ManaColor fromChar(char c) {
        return switch (Character.toUpperCase(c)) {
            case 'W' -> WHITE;
            case 'U' -> BLUE;
            case 'B' -> BLACK;
            case 'R' -> RED;
            case 'G' -> GREEN;
            case 'C' -> COLORLESS;
            default -> throw new IllegalArgumentException("Invalid mana color character: " + c);
        };
    }

    /**
     * Parse a ManaColor from a string (single character).
     * Used for Jackson deserialization.
     */
    public static ManaColor fromString(String s) {
        if (s == null || s.isEmpty()) {
            throw new IllegalArgumentException("Mana color string cannot be null or empty");
        }
        return fromChar(s.charAt(0));
    }
}

