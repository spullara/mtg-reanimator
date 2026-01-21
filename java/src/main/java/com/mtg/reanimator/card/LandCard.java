package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.ArrayList;
import java.util.List;

/**
 * Land card.
 * Matches the Rust LandCard struct.
 */
public class LandCard {
    @JsonProperty("name")
    private String name;

    @JsonProperty("mana_value")
    private int manaValue;

    @JsonProperty("subtype")
    private LandSubtype subtype;

    @JsonProperty("enters_tapped")
    private boolean entersTapped;

    @JsonProperty("colors")
    private List<ManaColor> colors = new ArrayList<>();

    @JsonProperty("has_surveil")
    private boolean hasSurveil;

    @JsonProperty("surveil_amount")
    private int surveilAmount;

    public LandCard() {
    }

    public String getName() {
        return name;
    }

    public int getManaValue() {
        return manaValue;
    }

    public LandSubtype getSubtype() {
        return subtype;
    }

    public boolean isEntersTapped() {
        return entersTapped;
    }

    public List<ManaColor> getColors() {
        return colors;
    }

    public boolean hasSurveil() {
        return hasSurveil;
    }

    public int getSurveilAmount() {
        return surveilAmount;
    }

    public void setName(String name) {
        this.name = name;
    }

    public void setManaValue(int manaValue) {
        this.manaValue = manaValue;
    }

    public void setSubtype(LandSubtype subtype) {
        this.subtype = subtype;
    }

    public void setEntersTapped(boolean entersTapped) {
        this.entersTapped = entersTapped;
    }

    public void setColors(List<ManaColor> colors) {
        this.colors = colors;
    }

    public void setHasSurveil(boolean hasSurveil) {
        this.hasSurveil = hasSurveil;
    }

    public void setSurveilAmount(int surveilAmount) {
        this.surveilAmount = surveilAmount;
    }
}

