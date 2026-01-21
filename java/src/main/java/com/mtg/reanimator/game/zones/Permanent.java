package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.Card;
import java.util.EnumMap;
import java.util.Map;

/**
 * A permanent on the battlefield with state tracking.
 * Matches the Rust Permanent struct.
 */
public class Permanent {
    private final Card card;
    private boolean tapped;
    private final int turnEntered;
    private final Map<CounterType, Integer> counters;
    private String chosenType;       // For Cavern of Souls
    private String chosenBasicType;  // For Multiversal Passage
    private String isCopyOf;         // For copying effects like Superior Spider-Man

    public Permanent(Card card, int turnEntered) {
        this.card = card;
        this.tapped = false;
        this.turnEntered = turnEntered;
        this.counters = new EnumMap<>(CounterType.class);
        this.chosenType = null;
        this.chosenBasicType = null;
        this.isCopyOf = null;
    }

    public Card getCard() {
        return card;
    }

    public boolean isTapped() {
        return tapped;
    }

    public void tap() {
        this.tapped = true;
    }

    public void untap() {
        this.tapped = false;
    }

    public int getTurnEntered() {
        return turnEntered;
    }

    /**
     * Check if this permanent has summoning sickness
     * (entered the battlefield this turn).
     */
    public boolean hasSummoningSickness(int currentTurn) {
        return turnEntered >= currentTurn;
    }

    public void addCounter(CounterType type, int amount) {
        counters.merge(type, amount, Integer::sum);
    }

    public boolean removeCounter(CounterType type, int amount) {
        Integer current = counters.get(type);
        if (current != null && current >= amount) {
            int remaining = current - amount;
            if (remaining == 0) {
                counters.remove(type);
            } else {
                counters.put(type, remaining);
            }
            return true;
        }
        return false;
    }

    public int getCounter(CounterType type) {
        return counters.getOrDefault(type, 0);
    }

    public String getChosenType() {
        return chosenType;
    }

    public void setChosenType(String chosenType) {
        this.chosenType = chosenType;
    }

    public String getChosenBasicType() {
        return chosenBasicType;
    }

    public void setChosenBasicType(String chosenBasicType) {
        this.chosenBasicType = chosenBasicType;
    }

    public String getIsCopyOf() {
        return isCopyOf;
    }

    public void setIsCopyOf(String isCopyOf) {
        this.isCopyOf = isCopyOf;
    }

    /**
     * Get the name of the underlying card.
     */
    public String getName() {
        return card.getName();
    }

    /**
     * Check if this permanent is a land.
     */
    public boolean isLand() {
        return card instanceof Card.Land;
    }

    /**
     * Get this permanent as a land, or null if not a land.
     */
    public Card.Land asLand() {
        return card instanceof Card.Land land ? land : null;
    }
}

