package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardDatabase;
import com.mtg.reanimator.card.CardDatabaseException;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/**
 * Deck representation and parsing.
 * Matches the Rust deck.rs implementation.
 */
public class Deck {
    private final List<Card> cards;
    private final String name;

    public Deck(List<Card> cards, String name) {
        this.cards = new ArrayList<>(cards);
        this.name = name;
    }

    /**
     * Load a deck from a file.
     * Format: "4 Card Name" per line, supports comments with # or //
     *
     * @param path Path to the deck file
     * @param db   Card database
     * @return Parsed deck
     * @throws DeckException if parsing fails
     */
    public static Deck loadFromFile(String path, CardDatabase db) throws DeckException {
        try {
            String content = Files.readString(Path.of(path));
            List<Card> cards = new ArrayList<>();
            String[] lines = content.split("\n");

            for (int lineNum = 0; lineNum < lines.length; lineNum++) {
                String line = lines[lineNum].trim();

                // Skip empty lines and comments
                if (line.isEmpty() || line.startsWith("#") || line.startsWith("//")) {
                    continue;
                }

                // Parse "N Card Name" format
                int spaceIdx = line.indexOf(' ');
                if (spaceIdx == -1) {
                    throw new DeckException("Invalid deck format at line " + (lineNum + 1)
                            + ": Expected format 'COUNT CARD_NAME'");
                }

                String countStr = line.substring(0, spaceIdx);
                String cardName = line.substring(spaceIdx + 1).trim();

                int count;
                try {
                    count = Integer.parseInt(countStr);
                } catch (NumberFormatException e) {
                    throw new DeckException("Invalid deck format at line " + (lineNum + 1)
                            + ": '" + countStr + "' is not a valid number");
                }

                // Get card from database
                try {
                    Card card = db.getCard(cardName);
                    for (int i = 0; i < count; i++) {
                        cards.add(card);
                    }
                } catch (CardDatabaseException e) {
                    throw new DeckException("Card not found at line " + (lineNum + 1) + ": " + cardName);
                }
            }

            // Extract deck name from path
            String fileName = Path.of(path).getFileName().toString();
            String deckName = fileName.endsWith(".txt")
                    ? fileName.substring(0, fileName.length() - 4)
                    : fileName;

            return new Deck(cards, deckName);
        } catch (IOException e) {
            throw new DeckException("Failed to read deck file: " + e.getMessage());
        }
    }

    public List<Card> getCards() {
        return new ArrayList<>(cards);
    }

    public int size() {
        return cards.size();
    }

    public String getName() {
        return name;
    }

    /**
     * Exception thrown when deck parsing fails.
     */
    public static class DeckException extends Exception {
        public DeckException(String message) {
            super(message);
        }
    }
}

