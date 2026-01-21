package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Mana cost for a card.
 * Matches the Rust ManaCost struct exactly.
 */
public class ManaCost {
    @JsonProperty("white")
    private int white;
    
    @JsonProperty("blue")
    private int blue;
    
    @JsonProperty("black")
    private int black;
    
    @JsonProperty("red")
    private int red;
    
    @JsonProperty("green")
    private int green;
    
    @JsonProperty("colorless")
    private int colorless;
    
    @JsonProperty("generic")
    private int generic;

    public ManaCost() {
        // Default constructor for Jackson
    }

    public ManaCost(int white, int blue, int black, int red, int green, int colorless, int generic) {
        this.white = white;
        this.blue = blue;
        this.black = black;
        this.red = red;
        this.green = green;
        this.colorless = colorless;
        this.generic = generic;
    }

    /**
     * Calculate the converted mana cost (total mana value).
     */
    public int getCMC() {
        return white + blue + black + red + green + colorless + generic;
    }

    /**
     * Get the amount of a specific color required.
     */
    public int getColorAmount(ManaColor color) {
        return switch (color) {
            case WHITE -> white;
            case BLUE -> blue;
            case BLACK -> black;
            case RED -> red;
            case GREEN -> green;
            case COLORLESS -> colorless;
        };
    }

    /**
     * Parse a mana cost string like "1BB", "2G", "3BBB".
     */
    public static ManaCost parse(String costString) {
        if (costString == null || costString.isEmpty()) {
            return new ManaCost();
        }

        int white = 0, blue = 0, black = 0, red = 0, green = 0, colorless = 0, generic = 0;
        StringBuilder genericBuilder = new StringBuilder();

        for (char c : costString.toCharArray()) {
            if (Character.isDigit(c)) {
                genericBuilder.append(c);
            } else {
                switch (Character.toUpperCase(c)) {
                    case 'W' -> white++;
                    case 'U' -> blue++;
                    case 'B' -> black++;
                    case 'R' -> red++;
                    case 'G' -> green++;
                    case 'C' -> colorless++;
                    default -> throw new IllegalArgumentException("Invalid mana symbol: " + c);
                }
            }
        }

        if (!genericBuilder.isEmpty()) {
            generic = Integer.parseInt(genericBuilder.toString());
        }

        return new ManaCost(white, blue, black, red, green, colorless, generic);
    }

    // Getters
    public int getWhite() { return white; }
    public int getBlue() { return blue; }
    public int getBlack() { return black; }
    public int getRed() { return red; }
    public int getGreen() { return green; }
    public int getColorless() { return colorless; }
    public int getGeneric() { return generic; }

    @Override
    public String toString() {
        StringBuilder sb = new StringBuilder();
        if (generic > 0) sb.append(generic);
        sb.append("W".repeat(white));
        sb.append("U".repeat(blue));
        sb.append("B".repeat(black));
        sb.append("R".repeat(red));
        sb.append("G".repeat(green));
        sb.append("C".repeat(colorless));
        return sb.isEmpty() ? "0" : sb.toString();
    }
}

