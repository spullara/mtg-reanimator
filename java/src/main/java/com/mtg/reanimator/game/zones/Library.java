package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.rng.GameRng;

import java.util.ArrayList;
import java.util.List;
import java.util.NoSuchElementException;
import java.util.Optional;

/**
 * Library (deck) - ordered stack of cards.
 * Matches the Rust Library struct.
 * Top of the library is at index 0.
 */
public class Library {
    private final List<Card> cards;

    public Library() {
        this.cards = new ArrayList<>();
    }

    public Library(int capacity) {
        this.cards = new ArrayList<>(capacity);
    }

    public void clear() {
        cards.clear();
    }

    public void addCard(Card card) {
        cards.add(card);
    }

    /**
     * Peek at the top card without removing it.
     */
    public Optional<Card> peekTop() {
        if (cards.isEmpty()) {
            return Optional.empty();
        }
        return Optional.of(cards.getFirst());
    }

    /**
     * Draw a card from the top of the library.
     * @return The drawn card
     * @throws NoSuchElementException if the library is empty
     */
    public Card draw() {
        if (cards.isEmpty()) {
            throw new NoSuchElementException("Cannot draw from empty library");
        }
        return cards.removeFirst();
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
     */
    public void shuffle(GameRng rng) {
        rng.shuffle(cards);
    }

    /**
     * Get an unmodifiable view of the cards.
     */
    public List<Card> getCards() {
        return List.copyOf(cards);
    }

    /**
     * Get direct access to the underlying card list (for mutation).
     */
    public List<Card> getCardsMutable() {
        return cards;
    }
}

