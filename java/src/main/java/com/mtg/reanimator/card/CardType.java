package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Card types in Magic.
 * Matches the Rust CardType enum exactly.
 */
public enum CardType {
    LAND("land"),
    CREATURE("creature"),
    INSTANT("instant"),
    SORCERY("sorcery"),
    ENCHANTMENT("enchantment"),
    SAGA("saga");

    private final String jsonValue;

    CardType(String jsonValue) {
        this.jsonValue = jsonValue;
    }

    @JsonValue
    public String getJsonValue() {
        return jsonValue;
    }

    public static CardType fromString(String value) {
        if (value == null) {
            throw new IllegalArgumentException("Card type cannot be null");
        }
        return switch (value.toLowerCase()) {
            case "land" -> LAND;
            case "creature" -> CREATURE;
            case "instant" -> INSTANT;
            case "sorcery" -> SORCERY;
            case "enchantment" -> ENCHANTMENT;
            case "saga" -> SAGA;
            default -> throw new IllegalArgumentException("Unknown card type: " + value);
        };
    }
}

