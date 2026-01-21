package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Land subtypes.
 * Matches the Rust LandSubtype enum exactly.
 */
public enum LandSubtype {
    BASIC("basic"),
    SHOCK("shock"),
    SURVEIL("surveil"),
    UTILITY("utility"),
    FASTLAND("fastland"),
    TOWN("town");

    private final String jsonValue;

    LandSubtype(String jsonValue) {
        this.jsonValue = jsonValue;
    }

    @JsonValue
    public String getJsonValue() {
        return jsonValue;
    }

    public static LandSubtype fromString(String value) {
        if (value == null) {
            return null;
        }
        return switch (value.toLowerCase()) {
            case "basic" -> BASIC;
            case "shock" -> SHOCK;
            case "surveil" -> SURVEIL;
            case "utility" -> UTILITY;
            case "fastland" -> FASTLAND;
            case "town" -> TOWN;
            default -> throw new IllegalArgumentException("Unknown land subtype: " + value);
        };
    }
}

