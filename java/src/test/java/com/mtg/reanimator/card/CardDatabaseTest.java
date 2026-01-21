package com.mtg.reanimator.card;

import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for CardDatabase.
 */
class CardDatabaseTest {

    private static CardDatabase db;

    @BeforeAll
    static void loadDatabase() throws CardDatabaseException {
        db = CardDatabase.fromFile("../cards.json");
    }

    @Test
    void testLoadCards() {
        assertTrue(db.cardCount() > 0, "Should have loaded cards");
    }

    @Test
    void testGetForest() throws CardDatabaseException {
        Card card = db.getCard("Forest");
        assertEquals("Forest", card.getName());
        assertEquals(CardType.LAND, card.getCardType());
        assertEquals(0, card.getManaValue());

        assertInstanceOf(Card.Land.class, card);
        Card.Land land = (Card.Land) card;
        assertEquals(LandSubtype.BASIC, land.getSubtype());
        assertFalse(land.isEntersTapped());
        assertTrue(land.getColors().contains(ManaColor.GREEN));
    }

    @Test
    void testGetCreatureCard() throws CardDatabaseException {
        Card card = db.getCard("Terror of the Peaks");
        assertEquals("Terror of the Peaks", card.getName());
        assertEquals(CardType.CREATURE, card.getCardType());
        assertEquals(5, card.getManaValue());

        assertInstanceOf(Card.Creature.class, card);
        Card.Creature creature = (Card.Creature) card;
        assertEquals(5, creature.getPower());
        assertEquals(4, creature.getToughness());
        assertTrue(creature.getCreatureTypes().contains("Dragon"));
        assertTrue(creature.getAbilities().contains("flying"));
    }

    @Test
    void testGetCreatureWithImpending() throws CardDatabaseException {
        Card card = db.getCard("Overlord of the Balemurk");
        assertEquals(CardType.CREATURE, card.getCardType());

        Card.Creature creature = (Card.Creature) card;
        assertTrue(creature.hasImpending());
        assertNotNull(creature.getImpendingCost());
        assertEquals(5, creature.getImpendingCounters());
    }

    @Test
    void testGetInstant() throws CardDatabaseException {
        Card card = db.getCard("Cache Grab");
        assertEquals(CardType.INSTANT, card.getCardType());
        assertEquals(2, card.getManaValue());

        assertInstanceOf(Card.Instant.class, card);
    }

    @Test
    void testGetSorcery() throws CardDatabaseException {
        Card card = db.getCard("Analyze the Pollen");
        assertEquals(CardType.SORCERY, card.getCardType());
        assertEquals(1, card.getManaValue());

        assertInstanceOf(Card.Sorcery.class, card);
    }

    @Test
    void testGetEnchantment() throws CardDatabaseException {
        Card card = db.getCard("Dredger's Insight");
        assertEquals(CardType.ENCHANTMENT, card.getCardType());

        assertInstanceOf(Card.Enchantment.class, card);
    }

    @Test
    void testGetSaga() throws CardDatabaseException {
        Card card = db.getCard("Awaken the Honored Dead");
        assertEquals(CardType.SAGA, card.getCardType());

        assertInstanceOf(Card.Saga.class, card);
        Card.Saga saga = (Card.Saga) card;
        assertEquals(3, saga.getChapters().size());
    }

    @Test
    void testCardNotFound() {
        assertThrows(CardDatabaseException.class, () -> db.getCard("Nonexistent Card"));
    }

    @Test
    void testHasCard() {
        assertTrue(db.hasCard("Forest"));
        assertTrue(db.hasCard("Swamp"));
        assertFalse(db.hasCard("Nonexistent Card"));
    }
}

