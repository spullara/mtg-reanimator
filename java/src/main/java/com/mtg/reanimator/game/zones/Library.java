package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.rng.GameRng;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.Iterator;
import java.util.List;
import java.util.NoSuchElementException;
import java.util.Optional;

/**
 * Library (deck) - ordered stack of cards.
 * Matches the Rust Library struct.
 * Top of the library is at index 0.
 *
 * Uses ArrayDeque internally for O(1) operations at both ends:
 * - draw() / removeFirst() is O(1) instead of O(n) with ArrayList
 * - putOnTop() / addFirst() is O(1) instead of O(n) with ArrayList
 */
public class Library {
    private Deque<Card> cards;

    public Library() {
        this.cards = new ArrayDeque<>();
    }

    public Library(int capacity) {
        this.cards = new ArrayDeque<>(capacity);
    }

    public void clear() {
        cards.clear();
    }

    public void addCard(Card card) {
        cards.addLast(card);
    }

    /**
     * Peek at the top card without removing it.
     */
    public Optional<Card> peekTop() {
        return Optional.ofNullable(cards.peekFirst());
    }

    /**
     * Draw a card from the top of the library.
     * @return The drawn card
     * @throws NoSuchElementException if the library is empty
     */
    public Card draw() {
        Card card = cards.pollFirst();
        if (card == null) {
            throw new NoSuchElementException("Cannot draw from empty library");
        }
        return card;
    }

    /**
     * Draw multiple cards from the library.
     * @param n Number of cards to draw
     * @return List of drawn cards (may be fewer if library runs out)
     */
    public List<Card> drawN(int n) {
        List<Card> drawn = new ArrayList<>(n);
        for (int i = 0; i < n && !cards.isEmpty(); i++) {
            drawn.add(cards.removeFirst());
        }
        return drawn;
    }

    /**
     * Mill cards from the top of the library.
     * @param count Number of cards to mill
     * @return List of milled cards
     */
    public List<Card> mill(int count) {
        List<Card> milled = new ArrayList<>(count);
        for (int i = 0; i < count && !cards.isEmpty(); i++) {
            milled.add(cards.removeFirst());
        }
        return milled;
    }

    /**
     * Put a card on top of the library.
     */
    public void putOnTop(Card card) {
        cards.addFirst(card);
    }

    /**
     * Put a card on the bottom of the library.
     */
    public void putOnBottom(Card card) {
        cards.addLast(card);
    }

    public int size() {
        return cards.size();
    }

    public boolean isEmpty() {
        return cards.isEmpty();
    }

    /**
     * Shuffle the library using the provided RNG.
     * Converts to list, shuffles, then rebuilds the deque.
     */
    public void shuffle(GameRng rng) {
        List<Card> list = new ArrayList<>(cards);
        rng.shuffle(list);
        cards = new ArrayDeque<>(list);
    }

    /**
     * Get an unmodifiable view of the cards.
     */
    public List<Card> getCards() {
        return List.copyOf(cards);
    }

    /**
     * Get a mutable list copy of the cards.
     * Note: Changes to the returned list do NOT affect the library.
     * Use findAndRemove() for searching and removing cards.
     * @deprecated Prefer using findAndRemove() instead for search-and-remove operations.
     */
    @Deprecated
    public List<Card> getCardsMutable() {
        return new ArrayList<>(cards);
    }

    /**
     * Find and remove a card by name from the library.
     * This is the preferred way to search for and remove a specific card.
     * @param cardName The name of the card to find
     * @return The removed card, or null if not found
     */
    public Card findAndRemove(String cardName) {
        Iterator<Card> iter = cards.iterator();
        while (iter.hasNext()) {
            Card card = iter.next();
            if (card.getName().equals(cardName)) {
                iter.remove();
                return card;
            }
        }
        return null;
    }

    /**
     * Replace the library contents with the given list.
     * Used after external modifications via getCardsMutable().
     * @param newCards The new card list
     */
    public void replaceWith(List<Card> newCards) {
        cards = new ArrayDeque<>(newCards);
    }
}

