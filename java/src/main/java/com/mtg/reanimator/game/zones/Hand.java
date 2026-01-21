package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.Card;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

/**
 * Hand - cards in hand.
 * Matches the Rust Hand struct.
 */
public class Hand {
    private final List<Card> cards;

    public Hand() {
        this.cards = new ArrayList<>();
    }

    public Hand(int capacity) {
        this.cards = new ArrayList<>(capacity);
    }

    public void clear() {
        cards.clear();
    }

    public void add(Card card) {
        cards.add(card);
    }

    public void addAll(List<Card> cardsToAdd) {
        cards.addAll(cardsToAdd);
    }

    /**
     * Remove a card by index.
     * @param index The index of the card to remove
     * @return The removed card, or null if index is out of bounds
     */
    public Card remove(int index) {
        if (index >= 0 && index < cards.size()) {
            return cards.remove(index);
        }
        return null;
    }

    /**
     * Remove a specific card from the hand.
     * @param card The card to remove
     * @return true if the card was found and removed
     */
    public boolean remove(Card card) {
        return cards.remove(card);
    }

    public int size() {
        return cards.size();
    }

    public boolean isEmpty() {
        return cards.isEmpty();
    }

    /**
     * Get an unmodifiable copy of the cards.
     */
    public List<Card> getCards() {
        return List.copyOf(cards);
    }

    /**
     * Check if the hand contains a card with the given name.
     */
    public boolean contains(String cardName) {
        return cards.stream().anyMatch(c -> c.getName().equals(cardName));
    }

    /**
     * Find a card by name.
     * @param name The card name to search for
     * @return The first card with the matching name, or empty if not found
     */
    public Optional<Card> findByName(String name) {
        return cards.stream()
                .filter(c -> c.getName().equals(name))
                .findFirst();
    }

    /**
     * Find the index of a card by name.
     * @param name The card name to search for
     * @return The index of the first matching card, or -1 if not found
     */
    public int findIndexByName(String name) {
        for (int i = 0; i < cards.size(); i++) {
            if (cards.get(i).getName().equals(name)) {
                return i;
            }
        }
        return -1;
    }

    /**
     * Get direct access to the underlying card list.
     */
    public List<Card> getCardsMutable() {
        return cards;
    }
}

