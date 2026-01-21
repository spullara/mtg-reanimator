package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.ArrayList;
import java.util.List;

/**
 * Spell card (Instant, Sorcery, Enchantment).
 * Matches the Rust SpellCard struct.
 */
public class SpellCard {
    @JsonProperty("name")
    private String name;

    @JsonProperty("mana_cost")
    private ManaCost manaCost;

    @JsonProperty("mana_value")
    private int manaValue;

    @JsonProperty("abilities")
    private List<String> abilities = new ArrayList<>();

    public SpellCard() {
        this.manaCost = new ManaCost();
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

    public List<String> getAbilities() {
        return abilities;
    }

    public void setName(String name) {
        this.name = name;
    }

    public void setManaCost(ManaCost manaCost) {
        this.manaCost = manaCost;
    }

    public void setManaValue(int manaValue) {
        this.manaValue = manaValue;
    }

    public void setAbilities(List<String> abilities) {
        this.abilities = abilities;
    }
}

