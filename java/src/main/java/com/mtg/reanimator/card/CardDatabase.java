package com.mtg.reanimator.card;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Card database that loads cards from JSON.
 * Matches the Rust CardDatabase struct.
 */
public class CardDatabase {
    private final Map<String, Card> cards;

    private CardDatabase(Map<String, Card> cards) {
        this.cards = cards;
    }

    /**
     * Load cards from a JSON file.
     */
    public static CardDatabase fromFile(String path) throws CardDatabaseException {
        try {
            String content = Files.readString(Path.of(path));
            return fromJson(content);
        } catch (IOException e) {
            throw new CardDatabaseException("IO error: " + e.getMessage(), e);
        }
    }

    /**
     * Load cards from a classpath resource.
     */
    public static CardDatabase fromResource(String resourcePath) throws CardDatabaseException {
        try (InputStream is = CardDatabase.class.getClassLoader().getResourceAsStream(resourcePath)) {
            if (is == null) {
                throw new CardDatabaseException("Resource not found: " + resourcePath);
            }
            ObjectMapper mapper = new ObjectMapper();
            List<Card> cardList = mapper.readValue(is, new TypeReference<List<Card>>() {});
            return fromCardList(cardList);
        } catch (IOException e) {
            throw new CardDatabaseException("JSON parsing error: " + e.getMessage(), e);
        }
    }

    /**
     * Load cards from a JSON string.
     */
    public static CardDatabase fromJson(String json) throws CardDatabaseException {
        try {
            ObjectMapper mapper = new ObjectMapper();
            List<Card> cardList = mapper.readValue(json, new TypeReference<List<Card>>() {});
            return fromCardList(cardList);
        } catch (IOException e) {
            throw new CardDatabaseException("JSON parsing error: " + e.getMessage(), e);
        }
    }

    private static CardDatabase fromCardList(List<Card> cardList) {
        Map<String, Card> cards = new HashMap<>();
        for (Card card : cardList) {
            cards.put(card.getName(), card);
        }
        return new CardDatabase(cards);
    }

    /**
     * Get a card by name.
     * @throws CardDatabaseException if the card is not found
     */
    public Card getCard(String name) throws CardDatabaseException {
        Card card = cards.get(name);
        if (card == null) {
            throw new CardDatabaseException("Card not found: " + name);
        }
        return card;
    }

    /**
     * Get total number of cards.
     */
    public int cardCount() {
        return cards.size();
    }

    /**
     * Check if a card exists.
     */
    public boolean hasCard(String name) {
        return cards.containsKey(name);
    }
}

