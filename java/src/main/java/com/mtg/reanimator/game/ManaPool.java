package com.mtg.reanimator.game;

import com.mtg.reanimator.card.ManaCost;
import com.mtg.reanimator.card.ManaColor;

/**
 * Mana pool tracking each color and colorless mana.
 * Matches the Rust ManaPool struct exactly.
 */
public class ManaPool {
    // Static array to avoid allocation in hot path
    private static final ManaColor[] GENERIC_PAYMENT_ORDER = {
        ManaColor.COLORLESS, ManaColor.WHITE, ManaColor.BLUE,
        ManaColor.BLACK, ManaColor.RED, ManaColor.GREEN
    };

    private int white;
    private int blue;
    private int black;
    private int red;
    private int green;
    private int colorless;

    public ManaPool() {
        this.white = 0;
        this.blue = 0;
        this.black = 0;
        this.red = 0;
        this.green = 0;
        this.colorless = 0;
    }

    /**
     * Add mana of a specific color.
     */
    public void addMana(ManaColor color, int amount) {
        switch (color) {
            case WHITE -> white += amount;
            case BLUE -> blue += amount;
            case BLACK -> black += amount;
            case RED -> red += amount;
            case GREEN -> green += amount;
            case COLORLESS -> colorless += amount;
        }
    }

    /**
     * Check if we can pay a mana cost.
     */
    public boolean canPay(ManaCost cost) {
        // Check colored requirements
        if (cost.getWhite() > white) return false;
        if (cost.getBlue() > blue) return false;
        if (cost.getBlack() > black) return false;
        if (cost.getRed() > red) return false;
        if (cost.getGreen() > green) return false;
        if (cost.getColorless() > colorless) return false;

        // Check if we have enough remaining for generic
        int remaining = (white - cost.getWhite())
            + (blue - cost.getBlue())
            + (black - cost.getBlack())
            + (red - cost.getRed())
            + (green - cost.getGreen())
            + (colorless - cost.getColorless());

        return remaining >= cost.getGeneric();
    }

    /**
     * Pay a mana cost from the pool.
     * @return true if successful, false if not enough mana
     */
    public boolean pay(ManaCost cost) {
        if (!canPay(cost)) {
            return false;
        }

        // Pay colored costs first
        white -= cost.getWhite();
        blue -= cost.getBlue();
        black -= cost.getBlack();
        red -= cost.getRed();
        green -= cost.getGreen();
        colorless -= cost.getColorless();

        // Pay generic with remaining mana (prefer colorless, then excess colors)
        int genericRemaining = cost.getGeneric();

        for (ManaColor color : GENERIC_PAYMENT_ORDER) {
            if (genericRemaining == 0) break;

            int available = switch (color) {
                case WHITE -> white;
                case BLUE -> blue;
                case BLACK -> black;
                case RED -> red;
                case GREEN -> green;
                case COLORLESS -> colorless;
            };

            int toPay = Math.min(available, genericRemaining);
            switch (color) {
                case WHITE -> white -= toPay;
                case BLUE -> blue -= toPay;
                case BLACK -> black -= toPay;
                case RED -> red -= toPay;
                case GREEN -> green -= toPay;
                case COLORLESS -> colorless -= toPay;
            }
            genericRemaining -= toPay;
        }

        return true;
    }

    /**
     * Clear the mana pool.
     */
    public void clear() {
        white = 0;
        blue = 0;
        black = 0;
        red = 0;
        green = 0;
        colorless = 0;
    }

    /**
     * Get mana of a specific color.
     */
    public int get(ManaColor color) {
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
     * Get total mana in the pool.
     */
    public int total() {
        return white + blue + black + red + green + colorless;
    }

    /**
     * Check if the pool is empty.
     */
    public boolean isEmpty() {
        return total() == 0;
    }

    // Getters
    public int getWhite() { return white; }
    public int getBlue() { return blue; }
    public int getBlack() { return black; }
    public int getRed() { return red; }
    public int getGreen() { return green; }
    public int getColorless() { return colorless; }
}

