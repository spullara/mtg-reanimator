package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.CardType;
import com.mtg.reanimator.card.LandSubtype;
import com.mtg.reanimator.card.ManaCost;
import com.mtg.reanimator.card.ManaColor;
import com.mtg.reanimator.game.GameState;
import com.mtg.reanimator.game.zones.Permanent;

import java.util.*;

/**
 * Decision engine for MTG Reanimator AI.
 * Handles land selection and mill return priorities.
 * Matches the Rust DecisionEngine implementation.
 */
public final class DecisionEngine {

    private DecisionEngine() {
        // Utility class - no instantiation
    }

    // ---- Constants ----
    private static final Set<String> COMBO_PIECES = Set.of(
            "Bringer of the Last Gift",
            "Terror of the Peaks"
    );

    private static final Set<String> MILL_ENABLERS = Set.of(
            "Town Greeter",
            "Overlord of the Balemurk",
            "Kiora, the Rising Tide"
    );

    private static final List<String> BLUE_PRODUCING_LANDS = List.of(
            "Watery Grave",
            "Undercity Sewers",
            "Gloomlake Verge",
            "Island"
    );

    /**
     * Choose which land to play from hand - sophisticated land selection logic.
     * Priority:
     * 1. Lands that enable casting a spell THIS turn (untapped + provides missing color)
     * 2. Lands that provide missing colors
     * 3. Surveil lands (for milling)
     * 4. Tapped lands (save untapped for later)
     * 5. Multi-color vs single-color
     *
     * @param hand Current hand cards
     * @param state Current game state
     * @return Index of the land to play, or empty if no land in hand
     */
    public static OptionalInt chooseLandToPlay(List<Card> hand, GameState state) {
        // Find all lands in hand with their indices
        List<IndexedCard> lands = new ArrayList<>();
        for (int i = 0; i < hand.size(); i++) {
            Card card = hand.get(i);
            if (card instanceof Card.Land) {
                lands.add(new IndexedCard(i, card));
            }
        }

        if (lands.isEmpty()) {
            return OptionalInt.empty();
        }

        // Calculate available mana and colors from untapped lands on battlefield
        Set<ManaColor> colorsAvailable = new HashSet<>();
        int manaAvailable = 0;

        for (Permanent perm : state.getBattlefield().getLands()) {
            if (!perm.isTapped()) {
                manaAvailable++;
                Card.Land land = perm.asLand();
                if (land != null) {
                    colorsAvailable.addAll(land.getColors());
                }
            }
        }

        int manaAfterLandDrop = manaAvailable + 1;

        // Find non-land cards (spells) in hand
        List<Card> spells = new ArrayList<>();
        for (int i = 0; i < hand.size(); i++) {
            Card c = hand.get(i);
            if (!(c instanceof Card.Land)) {
                spells.add(c);
            }
        }

        // Calculate missing colors needed for spells
        Set<ManaColor> missingColors = new HashSet<>();
        for (Card spell : spells) {
            addMissingColors(spell, colorsAvailable, missingColors);
        }

        // Sort lands by priority
        Set<ManaColor> finalColorsAvailable = colorsAvailable;
        lands.sort((a, b) -> compareLands(
                (Card.Land) a.card(),
                (Card.Land) b.card(),
                state,
                finalColorsAvailable,
                missingColors,
                spells,
                manaAfterLandDrop
        ));

        return OptionalInt.of(lands.get(0).index());
    }

    private static int compareLands(
            Card.Land a,
            Card.Land b,
            GameState state,
            Set<ManaColor> colorsAvailable,
            Set<ManaColor> missingColors,
            List<Card> spells,
            int manaAfterLandDrop
    ) {
        boolean aTapped = entersTapped(a, state);
        boolean bTapped = entersTapped(b, state);
        boolean aCanCast = canCastSpellThisTurn(a, aTapped, colorsAvailable, spells, manaAfterLandDrop);
        boolean bCanCast = canCastSpellThisTurn(b, bTapped, colorsAvailable, spells, manaAfterLandDrop);

        // Priority 1: Can cast a spell this turn
        if (aCanCast != bCanCast) {
            return aCanCast ? -1 : 1;
        }

        // If neither can cast this turn
        if (!aCanCast && !bCanCast) {
            boolean aMissing = providesMissingColor(a, missingColors);
            boolean bMissing = providesMissingColor(b, missingColors);
            if (aMissing != bMissing) {
                return aMissing ? -1 : 1;
            }

            // Prefer surveil lands
            if (a.hasSurveil() != b.hasSurveil()) {
                return a.hasSurveil() ? -1 : 1;
            }

            // Prefer tapped lands (save untapped for when we need them)
            if (aTapped != bTapped) {
                return aTapped ? -1 : 1;
            }
            return 0;
        }

        // Both can cast - prefer surveil, then more colors
        if (a.hasSurveil() != b.hasSurveil()) {
            return a.hasSurveil() ? -1 : 1;
        }

        // Prefer lands with more colors (more flexible)
        return Integer.compare(b.getColors().size(), a.getColors().size());
    }

    /**
     * Check if a land enters the battlefield tapped.
     */
    private static boolean entersTapped(Card.Land land, GameState state) {
        LandSubtype subtype = land.getSubtype();
        if (subtype == null) {
            return land.isEntersTapped();
        }

        return switch (subtype) {
            case FASTLAND -> {
                // Tapped if 3+ lands already on battlefield
                int landCount = state.getBattlefield().getLands().size();
                yield landCount >= 3;
            }
            case TOWN -> {
                // Tapped on turn 4+
                yield state.getTurn() > 3;
            }
            case UTILITY -> {
                // Verge lands: simplified to always enter untapped
                if (land.getName().endsWith("Verge")) {
                    yield false;
                }
                yield land.isEntersTapped();
            }
            default -> land.isEntersTapped();
        };
    }

    /**
     * Check if playing this land enables casting a spell this turn.
     */
    private static boolean canCastSpellThisTurn(
            Card.Land land,
            boolean entersTapped,
            Set<ManaColor> colorsAvailable,
            List<Card> spells,
            int manaAfterLandDrop
    ) {
        if (entersTapped) {
            return false;
        }

        // Calculate colors after playing this land
        Set<ManaColor> colorsAfter = new HashSet<>(colorsAvailable);
        colorsAfter.addAll(land.getColors());

        // Check if any spell can be cast
        for (Card spell : spells) {
            if (spell.getManaValue() <= manaAfterLandDrop && hasColorsForSpell(spell, colorsAfter)) {
                return true;
            }
        }
        return false;
    }

    /**
     * Check if a land provides any of the missing colors.
     */
    private static boolean providesMissingColor(Card.Land land, Set<ManaColor> missingColors) {
        for (ManaColor color : land.getColors()) {
            if (missingColors.contains(color)) {
                return true;
            }
        }
        return false;
    }

    /**
     * Add missing colors for a spell to the set.
     */
    private static void addMissingColors(Card spell, Set<ManaColor> available, Set<ManaColor> missing) {
        ManaCost cost = getManaCost(spell);
        if (cost == null) {
            return;
        }

        if (cost.getWhite() > 0 && !available.contains(ManaColor.WHITE)) {
            missing.add(ManaColor.WHITE);
        }
        if (cost.getBlue() > 0 && !available.contains(ManaColor.BLUE)) {
            missing.add(ManaColor.BLUE);
        }
        if (cost.getBlack() > 0 && !available.contains(ManaColor.BLACK)) {
            missing.add(ManaColor.BLACK);
        }
        if (cost.getRed() > 0 && !available.contains(ManaColor.RED)) {
            missing.add(ManaColor.RED);
        }
        if (cost.getGreen() > 0 && !available.contains(ManaColor.GREEN)) {
            missing.add(ManaColor.GREEN);
        }
    }

    /**
     * Check if the available colors can cast the spell.
     */
    private static boolean hasColorsForSpell(Card spell, Set<ManaColor> colors) {
        ManaCost cost = getManaCost(spell);
        if (cost == null) {
            return true; // Lands don't have mana costs
        }

        return (cost.getWhite() == 0 || colors.contains(ManaColor.WHITE))
                && (cost.getBlue() == 0 || colors.contains(ManaColor.BLUE))
                && (cost.getBlack() == 0 || colors.contains(ManaColor.BLACK))
                && (cost.getRed() == 0 || colors.contains(ManaColor.RED))
                && (cost.getGreen() == 0 || colors.contains(ManaColor.GREEN));
    }

    /**
     * Get the mana cost of a card.
     */
    private static ManaCost getManaCost(Card card) {
        return switch (card) {
            case Card.Creature c -> c.getManaCost();
            case Card.Instant i -> i.getManaCost();
            case Card.Sorcery s -> s.getManaCost();
            case Card.Enchantment e -> e.getManaCost();
            case Card.Saga sa -> sa.getManaCost();
            case Card.Land l -> null;
        };
    }

    /**
     * Select the best card from a milled set based on game state priorities.
     * NEVER returns Bringer or Terror - they must stay in graveyard for reanimation.
     *
     * @param cards The milled cards to choose from
     * @param state Current game state
     * @return The best card to return, or empty if none suitable
     */
    public static Optional<Card> selectBestFromMill(List<Card> cards, GameState state) {
        if (cards.isEmpty()) {
            return Optional.empty();
        }

        boolean hasSpiderInHand = state.getHand().contains("Superior Spider-Man");
        boolean hasBringerInHand = state.getHand().contains("Bringer of the Last Gift");
        int landCount = state.getBattlefield().getLands().size();
        int landsInHand = 0;
        List<Card> handCards = state.getHand().getCards();
        for (int i = 0; i < handCards.size(); i++) {
            if (handCards.get(i) instanceof Card.Land) {
                landsInHand++;
            }
        }

        // Priority 1: Superior Spider-Man (unless we already have one)
        if (!hasSpiderInHand) {
            for (Card card : cards) {
                if ("Superior Spider-Man".equals(card.getName())) {
                    return Optional.of(card);
                }
            }
        }

        // Priority 2: Kiora if Bringer is stuck in hand
        if (hasBringerInHand) {
            for (Card card : cards) {
                if ("Kiora, the Rising Tide".equals(card.getName())) {
                    return Optional.of(card);
                }
            }
        }

        // Priority 3: Land if desperate (<=1 land on field, 0 in hand)
        if (landCount <= 1 && landsInHand == 0) {
            for (Card card : cards) {
                if (card instanceof Card.Land) {
                    return Optional.of(card);
                }
            }
        }

        // Priority 4: Mill enablers
        for (Card card : cards) {
            if (card instanceof Card.Creature && MILL_ENABLERS.contains(card.getName())) {
                return Optional.of(card);
            }
        }

        // Priority 5: Land if < 4 lands
        if (landCount < 4) {
            for (Card card : cards) {
                if (card instanceof Card.Land) {
                    return Optional.of(card);
                }
            }
        }

        // Priority 6: Non-combo creature
        for (Card card : cards) {
            if (card instanceof Card.Creature && !COMBO_PIECES.contains(card.getName())) {
                return Optional.of(card);
            }
        }

        // Priority 7: Any permanent except combo pieces
        for (Card card : cards) {
            if (!(card instanceof Card.Instant) && !(card instanceof Card.Sorcery)
                    && !COMBO_PIECES.contains(card.getName())) {
                return Optional.of(card);
            }
        }

        return Optional.empty();
    }

    /**
     * Choose which card to return from mill by index.
     * Priority order:
     * 1. Spider-Man (combo piece - need in hand)
     * 2. Kiora (draw/discard engine)
     * 3. Blue-producing lands
     * 4. Non-basic lands
     * 5. Basic lands
     * 6. Non-combo creatures
     *
     * @param milled The milled cards
     * @param cardType The type of card to prefer (unused in current impl, for future expansion)
     * @return Index of the card to return, or empty if none suitable
     */
    public static OptionalInt chooseMillReturn(List<Card> milled, CardType cardType) {
        // Priority 1: Spider-Man
        for (int i = 0; i < milled.size(); i++) {
            if ("Superior Spider-Man".equals(milled.get(i).getName())) {
                return OptionalInt.of(i);
            }
        }

        // Priority 2: Kiora
        for (int i = 0; i < milled.size(); i++) {
            if ("Kiora, the Rising Tide".equals(milled.get(i).getName())) {
                return OptionalInt.of(i);
            }
        }

        // Priority 3: Blue-producing lands
        for (int i = 0; i < milled.size(); i++) {
            Card card = milled.get(i);
            if (card instanceof Card.Land land) {
                if (BLUE_PRODUCING_LANDS.contains(land.getName())) {
                    return OptionalInt.of(i);
                }
            }
        }

        // Priority 4: Non-basic lands
        for (int i = 0; i < milled.size(); i++) {
            Card card = milled.get(i);
            if (card instanceof Card.Land land) {
                if (land.getSubtype() != LandSubtype.BASIC) {
                    return OptionalInt.of(i);
                }
            }
        }

        // Priority 5: Basic lands
        for (int i = 0; i < milled.size(); i++) {
            if (milled.get(i) instanceof Card.Land) {
                return OptionalInt.of(i);
            }
        }

        // Priority 6: Non-combo creatures
        for (int i = 0; i < milled.size(); i++) {
            Card card = milled.get(i);
            if (card instanceof Card.Creature && !COMBO_PIECES.contains(card.getName())) {
                return OptionalInt.of(i);
            }
        }

        return OptionalInt.empty();
    }

    /**
     * Helper record for tracking card indices.
     */
    private record IndexedCard(int index, Card card) {}
}
