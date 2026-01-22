package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Base card properties shared by all card types.
 * Matches the Rust BaseCard struct.
 */
public class BaseCard {
    @JsonProperty("name")
    private String name;

    @JsonProperty("mana_cost")
    private ManaCost manaCost;

    @JsonProperty("mana_value")
    private int manaValue;

    public BaseCard() {
        this.manaCost = new ManaCost();
    }

    public BaseCard(String name, ManaCost manaCost, int manaValue) {
        // Intern the string for faster == comparisons and reduced memory
        this.name = name != null ? name.intern() : null;
        this.manaCost = manaCost != null ? manaCost : new ManaCost();
        this.manaValue = manaValue;
    }

    public String getName() {
        return name;
    }

    public ManaCost getManaCost() {
        return manaCost;
    }

    public int getManaValue() {
        return manaValue;
    }

    public void setName(String name) {
        // Intern the string for faster == comparisons and reduced memory
        this.name = name != null ? name.intern() : null;
    }

    public void setManaCost(ManaCost manaCost) {
        this.manaCost = manaCost;
    }

    public void setManaValue(int manaValue) {
        this.manaValue = manaValue;
    }
}

