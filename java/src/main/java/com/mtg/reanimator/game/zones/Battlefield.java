package com.mtg.reanimator.game.zones;

import com.mtg.reanimator.card.CardType;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

/**
 * Battlefield - permanents in play.
 * Matches the Rust Battlefield struct.
 */
public class Battlefield {
    private final List<Permanent> permanents;

    public Battlefield() {
        this.permanents = new ArrayList<>();
    }

    public Battlefield(int capacity) {
        this.permanents = new ArrayList<>(capacity);
    }

    public void clear() {
        permanents.clear();
    }

    public void add(Permanent permanent) {
        permanents.add(permanent);
    }

    public void remove(Permanent permanent) {
        permanents.remove(permanent);
    }

    /**
     * Remove a permanent by index.
     * @param index The index of the permanent to remove
     * @return The removed permanent, or null if index is out of bounds
     */
    public Permanent remove(int index) {
        if (index >= 0 && index < permanents.size()) {
            return permanents.remove(index);
        }
        return null;
    }

    /**
     * Get an unmodifiable copy of all permanents.
     */
    public List<Permanent> getAll() {
        return List.copyOf(permanents);
    }

    /**
     * Get all land permanents.
     */
    public List<Permanent> getLands() {
        return permanents.stream()
                .filter(Permanent::isLand)
                .toList();
    }

    /**
     * Get all untapped land permanents.
     */
    public List<Permanent> getUntappedLands() {
        return permanents.stream()
                .filter(p -> p.isLand() && !p.isTapped())
                .toList();
    }

    /**
     * Get all creature permanents.
     */
    public List<Permanent> getCreatures() {
        return permanents.stream()
                .filter(p -> p.getCard().getCardType() == CardType.CREATURE)
                .toList();
    }

    /**
     * Count permanents with the given name.
     */
    public int countByName(String name) {
        return (int) permanents.stream()
                .filter(p -> p.getName().equals(name))
                .count();
    }

    /**
     * Find a permanent by name.
     */
    public Optional<Permanent> findByName(String name) {
        return permanents.stream()
                .filter(p -> p.getName().equals(name))
                .findFirst();
    }

    /**
     * Untap all permanents.
     */
    public void untapAll() {
        permanents.forEach(Permanent::untap);
    }

    /**
     * Get direct access to the underlying permanent list (for mutation).
     * Note: This returns a mutable reference - use with care.
     */
    public List<Permanent> getPermanentsMutable() {
        return permanents;
    }

    /**
     * Get direct access to the underlying permanent list.
     * Alias for getPermanentsMutable() for compatibility with Rust port.
     */
    public List<Permanent> getPermanents() {
        return permanents;
    }

    /**
     * Add a permanent to the battlefield.
     * Alias for add() for compatibility with Rust port.
     */
    public void addPermanent(Permanent permanent) {
        permanents.add(permanent);
    }

    /**
     * Remove a permanent by index.
     * Alias for remove(int) for compatibility with Rust port.
     */
    public Permanent removePermanent(int index) {
        return remove(index);
    }

    public int size() {
        return permanents.size();
    }

    public boolean isEmpty() {
        return permanents.isEmpty();
    }
}

