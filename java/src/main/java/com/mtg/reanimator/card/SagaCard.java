package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.ArrayList;
import java.util.List;

/**
 * Saga card.
 * Matches the Rust SagaCard struct.
 */
public class SagaCard {
    @JsonProperty("name")
    private String name;

    @JsonProperty("mana_cost")
    private ManaCost manaCost;

    @JsonProperty("mana_value")
    private int manaValue;

    @JsonProperty("chapters")
    private List<String> chapters = new ArrayList<>();

    public SagaCard() {
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

    public List<String> getChapters() {
        return chapters;
    }

    public void setName(String name) {
        this.name = name != null ? name.intern() : null;
    }

    public void setManaCost(ManaCost manaCost) {
        this.manaCost = manaCost;
    }

    public void setManaValue(int manaValue) {
        this.manaValue = manaValue;
    }

    public void setChapters(List<String> chapters) {
        this.chapters = chapters;
    }
}

