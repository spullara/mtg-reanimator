package com.mtg.reanimator.card;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for ManaCost parsing and CMC calculation.
 */
class ManaCostTest {

    @Test
    void testParseSingleColor() {
        ManaCost cost = ManaCost.parse("G");
        assertEquals(0, cost.getGeneric());
        assertEquals(1, cost.getGreen());
        assertEquals(0, cost.getBlack());
        assertEquals(1, cost.getCMC());
    }

    @Test
    void testParseGenericAndColor() {
        ManaCost cost = ManaCost.parse("2G");
        assertEquals(2, cost.getGeneric());
        assertEquals(1, cost.getGreen());
        assertEquals(3, cost.getCMC());
    }

    @Test
    void testParseMultipleColors() {
        ManaCost cost = ManaCost.parse("1BB");
        assertEquals(1, cost.getGeneric());
        assertEquals(2, cost.getBlack());
        assertEquals(3, cost.getCMC());
    }

    @Test
    void testParseComplexCost() {
        ManaCost cost = ManaCost.parse("3BBB");
        assertEquals(3, cost.getGeneric());
        assertEquals(3, cost.getBlack());
        assertEquals(6, cost.getCMC());
    }

    @Test
    void testParseMultiColor() {
        ManaCost cost = ManaCost.parse("UBG");
        assertEquals(0, cost.getGeneric());
        assertEquals(1, cost.getBlue());
        assertEquals(1, cost.getBlack());
        assertEquals(1, cost.getGreen());
        assertEquals(3, cost.getCMC());
    }

    @Test
    void testParseEmptyString() {
        ManaCost cost = ManaCost.parse("");
        assertEquals(0, cost.getCMC());
    }

    @Test
    void testParseNull() {
        ManaCost cost = ManaCost.parse(null);
        assertEquals(0, cost.getCMC());
    }

    @Test
    void testToString() {
        ManaCost cost = ManaCost.parse("2BB");
        assertEquals("2BB", cost.toString());
    }

    @Test
    void testGetColorAmount() {
        ManaCost cost = ManaCost.parse("1UUBB");
        assertEquals(2, cost.getColorAmount(ManaColor.BLUE));
        assertEquals(2, cost.getColorAmount(ManaColor.BLACK));
        assertEquals(0, cost.getColorAmount(ManaColor.RED));
    }

    @Test
    void testColorless() {
        ManaCost cost = ManaCost.parse("2CC");
        assertEquals(2, cost.getGeneric());
        assertEquals(2, cost.getColorless());
        assertEquals(4, cost.getCMC());
    }
}

