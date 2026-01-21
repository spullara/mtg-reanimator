package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.ArrayList;
import java.util.List;

/**
 * Creature card.
 * Matches the Rust CreatureCard struct.
 */
public class CreatureCard {
    @JsonProperty("name")
    private String name;

    @JsonProperty("mana_cost")
    private ManaCost manaCost;

    @JsonProperty("mana_value")
    private int manaValue;

    @JsonProperty("power")
    private int power;

    @JsonProperty("toughness")
    private int toughness;

    @JsonProperty("is_legendary")
    private boolean isLegendary;

    @JsonProperty("creature_types")
    private List<String> creatureTypes = new ArrayList<>();

    @JsonProperty("abilities")
    private List<String> abilities = new ArrayList<>();

    @JsonProperty("impending_cost")
    private ManaCost impendingCost;

    @JsonProperty("impending_counters")
    private Integer impendingCounters;

    public CreatureCard() {
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

    public int getPower() {
        return power;
    }

    public int getToughness() {
        return toughness;
    }

    public boolean isLegendary() {
        return isLegendary;
    }

    public List<String> getCreatureTypes() {
        return creatureTypes;
    }

    public List<String> getAbilities() {
        return abilities;
    }

    public ManaCost getImpendingCost() {
        return impendingCost;
    }

    public Integer getImpendingCounters() {
        return impendingCounters;
    }

    public boolean hasImpending() {
        return impendingCost != null && impendingCounters != null;
    }

    // Setters for Jackson
    public void setName(String name) { this.name = name; }
    public void setManaCost(ManaCost manaCost) { this.manaCost = manaCost; }
    public void setManaValue(int manaValue) { this.manaValue = manaValue; }
    public void setPower(int power) { this.power = power; }
    public void setToughness(int toughness) { this.toughness = toughness; }
    public void setLegendary(boolean legendary) { isLegendary = legendary; }
    public void setCreatureTypes(List<String> creatureTypes) { this.creatureTypes = creatureTypes; }
    public void setAbilities(List<String> abilities) { this.abilities = abilities; }
    public void setImpendingCost(ManaCost impendingCost) { this.impendingCost = impendingCost; }
    public void setImpendingCounters(Integer impendingCounters) { this.impendingCounters = impendingCounters; }
}

