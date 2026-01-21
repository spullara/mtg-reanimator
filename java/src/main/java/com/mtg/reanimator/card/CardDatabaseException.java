package com.mtg.reanimator.card;

/**
 * Exception thrown by CardDatabase operations.
 * Matches the Rust CardDatabaseError enum.
 */
public class CardDatabaseException extends Exception {
    public CardDatabaseException(String message) {
        super(message);
    }

    public CardDatabaseException(String message, Throwable cause) {
        super(message, cause);
    }
}

