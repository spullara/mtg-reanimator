package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardType;
import com.mtg.reanimator.rng.GameRng;

import java.util.ArrayList;
import java.util.List;

/**
 * Mulligan resolution for MTG simulation.
 * Implements BO1 hand smoother algorithm and mulligan decisions.
 * Matches the Rust mulligan.rs implementation.
 */
public final class MulliganResolver {

    private MulliganResolver() {
        // Utility class
    }

    /**
     * Count the number of lands in a hand.
     */
    public static int countLands(List<Card> hand) {
        int count = 0;
        for (int i = 0; i < hand.size(); i++) {
            if (hand.get(i).getCardType() == CardType.LAND) {
                count++;
            }
        }
        return count;
    }

    /**
     * Check if a card is a mill/surveil enabler.
     */
    public static boolean isMillEnabler(Card card) {
        String name = card.getName();
        return switch (name) {
            case "Stitcher's Supplier",
                 "Teachings of the Kirin",
                 "Town Greeter",
                 "Overlord of the Balemurk",
                 "Kiora, the Rising Tide",
                 "Cache Grab",
                 "Dredger's Insight",
                 "Awaken the Honored Dead" -> true;
            default -> false;
        };
    }

    /**
     * Check if a card is a playable early spell (mana value <= 3, not a land).
     */
    public static boolean isPlayableEarlySpell(Card card) {
        return card.getManaValue() <= 3 && card.getCardType() != CardType.LAND;
    }

    /**
     * Decide whether to mulligan a hand.
     * Keep hands with:
     * - 2-5 lands AND at least one playable early spell
     * - Mill/surveil enabler with 2+ lands
     * At 4 cards or fewer, keep almost anything with 2+ lands.
     */
    public static boolean shouldMulligan(List<Card> hand, int mulliganCount) {
        int lands = countLands(hand);

        // At 4 cards or fewer, keep almost anything with 2+ lands
        if (hand.size() <= 4) {
            return lands < 2;
        }

        // Check for mill enablers - always keep if we have one with enough lands
        boolean hasMillEnabler = false;
        for (int i = 0; i < hand.size(); i++) {
            if (isMillEnabler(hand.get(i))) {
                hasMillEnabler = true;
                break;
            }
        }
        if (hasMillEnabler) {
            return lands < 2;
        }

        // Check for playable early spells
        boolean hasEarlySpell = false;
        for (int i = 0; i < hand.size(); i++) {
            if (isPlayableEarlySpell(hand.get(i))) {
                hasEarlySpell = true;
                break;
            }
        }

        // Keep if we have 2-5 lands and at least one early spell
        if (lands >= 2 && lands <= 5 && hasEarlySpell) {
            return false;
        }

        // Mulligan if we don't have enough lands or playable spells
        return lands < 2 || !hasEarlySpell;
    }

    /**
     * Scry after mulligan - decide which cards to put on bottom.
     * Bottom: Bringer/Terror (want in graveyard)
     * Bottom: lands if hand has 3+ lands
     * Bottom: expensive spells if missing lands
     */
    private static void scryAfterMulligan(List<Card> library, List<Card> hand, int scryCount) {
        if (scryCount == 0 || library.isEmpty()) {
            return;
        }

        int handLands = countLands(hand);
        List<Card> toBottom = new ArrayList<>();
        List<Card> toTop = new ArrayList<>();

        // Look at top scryCount cards
        int actualScry = Math.min(scryCount, library.size());
        List<Card> scryCards = new ArrayList<>();
        for (int i = 0; i < actualScry; i++) {
            scryCards.add(library.removeFirst());
        }

        for (Card card : scryCards) {
            String name = card.getName();

            // Always bottom Bringer/Terror (want in graveyard, not hand)
            if ("Bringer of the Last Gift".equals(name) || "Terror of the Peaks".equals(name)) {
                toBottom.add(card);
            }
            // Bottom lands if we have enough in hand
            else if (card.getCardType() == CardType.LAND && handLands >= 3) {
                toBottom.add(card);
            }
            // Bottom expensive spells if we're missing lands
            else if (card.getManaValue() >= 4 && handLands < 2) {
                toBottom.add(card);
            } else {
                toTop.add(card);
            }
        }

        // Reconstruct library: top cards at front, then rest, then bottom cards at end
        List<Card> remaining = new ArrayList<>(library);
        library.clear();
        library.addAll(toTop);
        library.addAll(remaining);
        library.addAll(toBottom);
    }

    /**
     * Mulligan to a smaller hand size, with scry.
     */
    private static List<Card> mulliganHand(List<Card> library, int handSize, GameRng rng) {
        List<Card> hand = new ArrayList<>(handSize);
        for (int i = 0; i < handSize && !library.isEmpty(); i++) {
            hand.add(library.removeFirst());
        }

        int lands = countLands(hand);
        if (lands < 2 && handSize > 4) {
            // Still bad, mulligan again
            library.addAll(hand);
            rng.shuffle(library);
            return mulliganHand(library, handSize - 1, rng);
        }

        // Scry for each card below 7
        int scryCount = 7 - handSize;
        if (scryCount > 0) {
            scryAfterMulligan(library, hand, scryCount);
        }

        return hand;
    }

    /**
     * Resolve mulligans starting from opening hand.
     * Uses BO1 hand smoother: draw two hands, pick one closer to ideal land count.
     * Returns the final hand after all mulligans and scries.
     *
     * @param library The library (deck) - will be modified
     * @param rng Random number generator
     * @return The final opening hand
     */
    public static List<Card> resolveMulligans(List<Card> library, GameRng rng) {
        // BO1 Hand Smoother: Draw two hands of 7
        List<Card> hand1 = new ArrayList<>(7);
        List<Card> hand2 = new ArrayList<>(7);

        for (int i = 0; i < 7 && !library.isEmpty(); i++) {
            hand1.add(library.removeFirst());
        }
        for (int i = 0; i < 7 && !library.isEmpty(); i++) {
            hand2.add(library.removeFirst());
        }

        int lands1 = countLands(hand1);
        int lands2 = countLands(hand2);

        List<Card> chosenHand;
        List<Card> rejectedHand;

        if (lands1 >= 2 && lands2 >= 2) {
            // Both hands have at least 2 lands, pick the one with fewer lands
            if (lands1 < lands2) {
                chosenHand = hand1;
                rejectedHand = hand2;
            } else if (lands2 < lands1) {
                chosenHand = hand2;
                rejectedHand = hand1;
            } else {
                // Same land count, random pick (matches TypeScript behavior)
                if (rng.next() < 0.5) {
                    chosenHand = hand1;
                    rejectedHand = hand2;
                } else {
                    chosenHand = hand2;
                    rejectedHand = hand1;
                }
            }
        } else if (lands1 >= 2) {
            chosenHand = hand1;
            rejectedHand = hand2;
        } else if (lands2 >= 2) {
            chosenHand = hand2;
            rejectedHand = hand1;
        } else {
            // Both hands have 0-1 lands, need to mulligan
            library.addAll(hand1);
            library.addAll(hand2);
            rng.shuffle(library);
            return mulliganHand(library, 6, rng);
        }

        // Put rejected hand back into library and shuffle
        library.addAll(rejectedHand);
        rng.shuffle(library);

        // Check if we need to mulligan the chosen hand
        int mulliganCount = 0;
        while (shouldMulligan(chosenHand, mulliganCount) && chosenHand.size() > 4) {
            int nextHandSize = chosenHand.size() - 1;
            library.addAll(chosenHand);
            rng.shuffle(library);
            chosenHand = mulliganHand(library, nextHandSize, rng);
            mulliganCount++;
        }

        return chosenHand;
    }

    /**
     * Draw an opening hand using BO1 hand smoother.
     * This is a convenience method that draws 3 hands and picks the one
     * closest to 3 lands (as specified in the task).
     *
     * Note: The current implementation follows the Rust reference which draws 2 hands.
     * This method provides the 3-hand variant specified in the task.
     */
    public static List<Card> drawOpeningHand(List<Card> library, GameRng rng) {
        // BO1 hand smoother: draw 3 hands, keep one closest to 3 lands
        List<List<Card>> hands = new ArrayList<>(3);

        for (int h = 0; h < 3; h++) {
            List<Card> hand = new ArrayList<>(7);
            for (int i = 0; i < 7 && !library.isEmpty(); i++) {
                hand.add(library.removeFirst());
            }
            hands.add(hand);
        }

        // Find hand with lowest land score (distance from 3 lands)
        int bestIndex = 0;
        int bestScore = Integer.MAX_VALUE;

        for (int i = 0; i < hands.size(); i++) {
            int lands = countLands(hands.get(i));
            int score = Math.abs(lands - 3);
            if (score < bestScore) {
                bestScore = score;
                bestIndex = i;
            }
            // On ties, keep the first hand (no change needed)
        }

        // Put rejected hands back into library
        for (int i = 0; i < hands.size(); i++) {
            if (i != bestIndex) {
                library.addAll(hands.get(i));
            }
        }
        rng.shuffle(library);

        return hands.get(bestIndex);
    }

    /**
     * Perform a mulligan: put hand back, shuffle, draw one fewer card.
     *
     * @param library The library
     * @param hand The current hand (will be cleared)
     * @param rng Random number generator
     * @return The new hand with one fewer card
     */
    public static List<Card> performMulligan(List<Card> library, List<Card> hand, GameRng rng) {
        int newHandSize = Math.max(1, hand.size() - 1);

        // Put hand back into library
        library.addAll(hand);
        rng.shuffle(library);

        // Draw one fewer card
        List<Card> newHand = new ArrayList<>(newHandSize);
        for (int i = 0; i < newHandSize && !library.isEmpty(); i++) {
            newHand.add(library.removeFirst());
        }

        // Scry for each card below 7
        int scryCount = 7 - newHandSize;
        if (scryCount > 0) {
            scryAfterMulligan(library, newHand, scryCount);
        }

        return newHand;
    }
}

