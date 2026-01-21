package com.mtg.reanimator.card;

import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;

/**
 * Unified card type - a sealed interface matching Rust's Card enum.
 * Uses Jackson polymorphic deserialization based on "card_type" field.
 */
@JsonTypeInfo(
    use = JsonTypeInfo.Id.NAME,
    include = JsonTypeInfo.As.PROPERTY,
    property = "card_type"
)
@JsonSubTypes({
    @JsonSubTypes.Type(value = Card.Land.class, name = "land"),
    @JsonSubTypes.Type(value = Card.Creature.class, name = "creature"),
    @JsonSubTypes.Type(value = Card.Instant.class, name = "instant"),
    @JsonSubTypes.Type(value = Card.Sorcery.class, name = "sorcery"),
    @JsonSubTypes.Type(value = Card.Enchantment.class, name = "enchantment"),
    @JsonSubTypes.Type(value = Card.Saga.class, name = "saga")
})
public sealed interface Card permits Card.Land, Card.Creature, Card.Instant, Card.Sorcery, Card.Enchantment, Card.Saga {

    String getName();
    int getManaValue();
    CardType getCardType();

    /**
     * Land card wrapper
     */
    final class Land extends LandCard implements Card {
        @Override
        public CardType getCardType() {
            return CardType.LAND;
        }
    }

    /**
     * Creature card wrapper
     */
    final class Creature extends CreatureCard implements Card {
        @Override
        public CardType getCardType() {
            return CardType.CREATURE;
        }
    }

    /**
     * Instant spell wrapper
     */
    final class Instant extends SpellCard implements Card {
        @Override
        public CardType getCardType() {
            return CardType.INSTANT;
        }
    }

    /**
     * Sorcery spell wrapper
     */
    final class Sorcery extends SpellCard implements Card {
        @Override
        public CardType getCardType() {
            return CardType.SORCERY;
        }
    }

    /**
     * Enchantment spell wrapper
     */
    final class Enchantment extends SpellCard implements Card {
        @Override
        public CardType getCardType() {
            return CardType.ENCHANTMENT;
        }
    }

    /**
     * Saga card wrapper
     */
    final class Saga extends SagaCard implements Card {
        @Override
        public CardType getCardType() {
            return CardType.SAGA;
        }
    }
}

