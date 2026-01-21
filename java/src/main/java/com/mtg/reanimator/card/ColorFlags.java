package com.mtg.reanimator.card;

/**
 * Bitflag representation of mana colors for fast operations.
 * Matches the Rust ColorFlags struct exactly.
 */
public class ColorFlags {
    public static final int WHITE = 1 << 0;     // 1
    public static final int BLUE = 1 << 1;      // 2
    public static final int BLACK = 1 << 2;     // 4
    public static final int RED = 1 << 3;       // 8
    public static final int GREEN = 1 << 4;     // 16
    public static final int COLORLESS = 1 << 5; // 32

    private int flags;

    public ColorFlags() {
        this.flags = 0;
    }

    public ColorFlags(int flags) {
        this.flags = flags;
    }

    public void insert(ManaColor color) {
        this.flags |= color.getFlag();
    }

    public boolean containsFlag(int flag) {
        return (this.flags & flag) != 0;
    }

    public boolean hasWhite() {
        return containsFlag(WHITE);
    }

    public boolean hasBlue() {
        return containsFlag(BLUE);
    }

    public boolean hasBlack() {
        return containsFlag(BLACK);
    }

    public boolean hasRed() {
        return containsFlag(RED);
    }

    public boolean hasGreen() {
        return containsFlag(GREEN);
    }

    public boolean hasColorless() {
        return containsFlag(COLORLESS);
    }

    public boolean isEmpty() {
        return flags == 0;
    }

    public int count() {
        return Integer.bitCount(flags);
    }

    public boolean contains(ManaColor color) {
        return containsFlag(color.getFlag());
    }

    /**
     * Get the first (any) color that's set, for generic mana payment.
     */
    public ManaColor firstColor() {
        if (hasWhite()) return ManaColor.WHITE;
        if (hasBlue()) return ManaColor.BLUE;
        if (hasBlack()) return ManaColor.BLACK;
        if (hasRed()) return ManaColor.RED;
        if (hasGreen()) return ManaColor.GREEN;
        if (hasColorless()) return ManaColor.COLORLESS;
        return null;
    }

    public int getFlags() {
        return flags;
    }
}

