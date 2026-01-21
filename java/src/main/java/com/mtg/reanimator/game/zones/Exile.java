package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.Card;

import java.util.ArrayList;
import java.util.List;

/**
 * Exile zone - exiled cards.
 * Matches the Rust Exile struct.
 */
public class Exile {
    private final List<Card> cards;

    public Exile() {
        this.cards = new ArrayList<>();
    }

    public Exile(int capacity) {
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
     * Get an unmodifiable copy of the exiled cards.
     */
    public List<Card> getCards() {
        return List.copyOf(cards);
    }

    /**
     * Remove a card by index.
     */
    public Card remove(int index) {
        if (index >= 0 && index < cards.size()) {
            return cards.remove(index);
        }
        return null;
    }

    /**
     * Get direct access to the underlying card list.
     */
    public List<Card> getCardsMutable() {
        return cards;
    }

    public int size() {
        return cards.size();
    }

    public boolean isEmpty() {
        return cards.isEmpty();
    }
}

