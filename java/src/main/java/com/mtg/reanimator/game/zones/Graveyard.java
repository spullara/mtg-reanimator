package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardType;

import java.util.ArrayList;
import java.util.List;

/**
 * Graveyard - discard pile (ordered stack).
 * Matches the Rust Graveyard struct.
 * Most recent cards are at the end.
 */
public class Graveyard {
    private final List<Card> cards;

    public Graveyard() {
        this.cards = new ArrayList<>();
    }

    public Graveyard(int capacity) {
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
     * Get an unmodifiable copy of the cards.
     */
    public List<Card> getCards() {
        return List.copyOf(cards);
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
     * Remove and return all cards from the graveyard.
     */
    public List<Card> removeAll() {
        List<Card> removed = new ArrayList<>(cards);
        cards.clear();
        return removed;
    }

    /**
     * Remove all creatures from the graveyard.
     */
    public void clearCreatures() {
        cards.removeIf(c -> c.getCardType() == CardType.CREATURE);
    }

    public int size() {
        return cards.size();
    }

    public boolean isEmpty() {
        return cards.isEmpty();
    }

    /**
     * Count the number of creature cards in the graveyard.
     */
    public int countCreatures() {
        int count = 0;
        for (int i = 0; i < cards.size(); i++) {
            if (cards.get(i).getCardType() == CardType.CREATURE) {
                count++;
            }
        }
        return count;
    }

    /**
     * Get the total power of all creature cards in the graveyard.
     */
    public int totalCreaturePower() {
        int totalPower = 0;
        for (int i = 0; i < cards.size(); i++) {
            Card card = cards.get(i);
            if (card instanceof Card.Creature creature) {
                totalPower += creature.getPower();
            }
        }
        return totalPower;
    }

    /**
     * Get all creature cards in the graveyard.
     */
    public List<Card> getCreatures() {
        List<Card> creatures = new ArrayList<>();
        for (int i = 0; i < cards.size(); i++) {
            Card card = cards.get(i);
            if (card.getCardType() == CardType.CREATURE) {
                creatures.add(card);
            }
        }
        return creatures;
    }

    /**
     * Get direct access to the underlying card list.
     */
    public List<Card> getCardsMutable() {
        return cards;
    }
}

