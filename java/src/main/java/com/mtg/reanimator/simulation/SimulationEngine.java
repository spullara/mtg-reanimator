package com.mtg.reanimator.simulation;

import com.mtg.reanimator.card.*;
import com.mtg.reanimator.game.*;
import com.mtg.reanimator.game.zones.*;
import com.mtg.reanimator.rng.GameRng;

import java.util.*;

/**
 * Main simulation engine for running MTG Reanimator games.
 * Matches the Rust engine.rs implementation exactly.
 */
public final class SimulationEngine {

    // Known mill enabler cards for mulligan decisions
    private static final Set<String> MILL_ENABLERS = Set.of(
        "Stitcher's Supplier",
        "Teachings of the Kirin",
        "Town Greeter",
        "Overlord of the Balemurk",
        "Kiora, the Rising Tide",
        "Cache Grab",
        "Dredger's Insight",
        "Awaken the Honored Dead"
    );

    // Land-finding spells
    private static final Set<String> LAND_FINDERS = Set.of(
        "Cache Grab",
        "Dredger's Insight",
        "Town Greeter"
    );

    // Known Demons in the game
    private static final Set<String> KNOWN_DEMONS = Set.of(
        "Bringer of the Last Gift"
    );

    private SimulationEngine() {
        // Utility class - prevent instantiation
    }

    // ==================== HELPER METHODS ====================

    /**
     * Check if any card in the list has the given name.
     * Uses identity comparison since card names are interned.
     */
    private static boolean hasCardNamed(List<Card> cards, String name) {
        for (int i = 0; i < cards.size(); i++) {
            if (cards.get(i).getName() == name) {
                return true;
            }
        }
        return false;
    }

    /**
     * Count creatures in the list excluding those with the given name.
     */
    private static int countCreaturesExcluding(List<Card> cards, String excludeName) {
        int count = 0;
        for (int i = 0; i < cards.size(); i++) {
            Card c = cards.get(i);
            if (c instanceof Card.Creature && c.getName() != excludeName) {
                count++;
            }
        }
        return count;
    }

    /**
     * Count untapped lands on the battlefield.
     */
    private static int countUntappedLands(Battlefield battlefield) {
        int count = 0;
        List<Permanent> permanents = battlefield.getPermanents();
        for (int i = 0; i < permanents.size(); i++) {
            Permanent p = permanents.get(i);
            if (p.isLand() && !p.isTapped()) {
                count++;
            }
        }
        return count;
    }

    /**
     * Check if any card has one of the given names.
     */
    private static boolean hasAnyCardNamed(List<Card> cards, String name1, String name2) {
        for (int i = 0; i < cards.size(); i++) {
            String name = cards.get(i).getName();
            if (name == name1 || name == name2) {
                return true;
            }
        }
        return false;
    }

    /**
     * Check if graveyard has any mill creature (Overlord, Kiora, or Town Greeter).
     */
    private static boolean hasMillCreatureInGraveyard(List<Card> cards) {
        for (int i = 0; i < cards.size(); i++) {
            String name = cards.get(i).getName();
            if (name == "Overlord of the Balemurk" || name == "Kiora, the Rising Tide" || name == "Town Greeter") {
                return true;
            }
        }
        return false;
    }

    /**
     * Count cards with the given name.
     */
    private static int countCardsNamed(List<Card> cards, String name) {
        int count = 0;
        for (int i = 0; i < cards.size(); i++) {
            if (cards.get(i).getName() == name) {
                count++;
            }
        }
        return count;
    }

    /**
     * Check if hand has lands.
     */
    private static boolean hasLandsInHand(List<Card> cards) {
        for (int i = 0; i < cards.size(); i++) {
            if (cards.get(i) instanceof Card.Land) {
                return true;
            }
        }
        return false;
    }

    // ==================== WIN CONDITION ====================

    /**
     * Check if the game has been won.
     */
    public static boolean checkWinCondition(GameState state) {
        return state.getOpponentLife() <= 0;
    }

    // ==================== COLOR AVAILABILITY ====================

    /**
     * Get available mana colors from battlefield lands.
     * Uses canTapForMana to correctly handle conditional lands.
     */
    public static ColorFlags getAvailableColors(GameState state) {
        ColorFlags colors = new ColorFlags();
        Battlefield battlefield = state.getBattlefield();

        for (Permanent permanent : battlefield.getPermanents()) {
            if (permanent.isLand()) {
                ColorFlags landColors = ManaUtils.getProducedColors(
                    permanent, battlefield, null, state.getLife()
                );
                // Merge colors using bitwise OR
                colors = new ColorFlags(colors.getFlags() | landColors.getFlags());
            }
        }

        return colors;
    }

    // ==================== ARDYN AND DEMON CHECKS ====================

    /**
     * Check if Ardyn, the Usurper is on the battlefield.
     */
    public static boolean hasArdynOnBattlefield(GameState state) {
        return state.getBattlefield().getPermanents().stream()
            .anyMatch(p -> CardNames.is(p.getName(), CardNames.ARDYN_THE_USURPER)
                || CardNames.is(p.getIsCopyOf(), CardNames.ARDYN_THE_USURPER));
    }

    /**
     * Check if a permanent is a Demon.
     */
    public static boolean isDemon(Permanent permanent) {
        Card card = permanent.getCard();

        // Check if the card itself is a Demon
        boolean cardIsDemon = false;
        if (card instanceof Card.Creature creature) {
            cardIsDemon = creature.getCreatureTypes().contains("Demon");
        }

        // Check if this is a copy of a known Demon
        String copyOf = permanent.getIsCopyOf();
        boolean copyIsDemon = copyOf != null && KNOWN_DEMONS.contains(copyOf);

        return cardIsDemon || copyIsDemon;
    }

    // ==================== MULLIGAN LOGIC ====================

    /**
     * Count lands in a list of cards.
     */
    private static int countLands(List<Card> cards) {
        return (int) cards.stream().filter(c -> c instanceof Card.Land).count();
    }

    /**
     * Check if a card is a mill/surveil enabler.
     */
    private static boolean isMillEnabler(Card card) {
        return MILL_ENABLERS.contains(card.getName());
    }

    /**
     * Check if a card is a playable early spell.
     */
    private static boolean isPlayableEarlySpell(Card card) {
        return card.getManaValue() <= 3 && !(card instanceof Card.Land);
    }

    /**
     * Decide whether to mulligan a hand.
     */
    public static boolean shouldMulligan(List<Card> hand, int mulliganCount) {
        int lands = countLands(hand);

        // At 4 cards or fewer, keep almost anything with 2+ lands
        if (hand.size() <= 4) {
            return lands < 2;
        }

        // Check for mill enablers - always keep if we have one
        if (hand.stream().anyMatch(SimulationEngine::isMillEnabler)) {
            return lands < 2;
        }

        // Check for playable early spells
        boolean hasEarlySpell = hand.stream().anyMatch(SimulationEngine::isPlayableEarlySpell);

        // Keep if we have 2-5 lands and at least one early spell
        if (lands >= 2 && lands <= 5 && hasEarlySpell) {
            return false;
        }

        // Mulligan if we don't have enough lands or playable spells
        return lands < 2 || !hasEarlySpell;
    }

    // ==================== SCRY AFTER MULLIGAN ====================

    /**
     * Scry after mulligan - decide which cards to put on bottom.
     * @param library The library (will be modified)
     * @param hand The player's hand
     * @param scryCount Number of cards to scry
     */
    private static void scryAfterMulligan(List<Card> library, List<Card> hand, int scryCount) {
        if (scryCount <= 0 || library.isEmpty()) {
            return;
        }

        int handLands = countLands(hand);
        List<Card> toBottom = new ArrayList<>();
        List<Card> toTop = new ArrayList<>();

        // Look at top scryCount cards
        int cardsToScry = Math.min(scryCount, library.size());
        List<Card> scryCards = new ArrayList<>(library.subList(0, cardsToScry));
        library.subList(0, cardsToScry).clear();

        for (Card card : scryCards) {
            String name = card.getName();

            // Always bottom Bringer/Terror (want in graveyard, not hand)
            if (name.equals("Bringer of the Last Gift") || name.equals("Terror of the Peaks")) {
                toBottom.add(card);
            }
            // Bottom lands if we have enough in hand
            else if (card instanceof Card.Land && handLands >= 3) {
                toBottom.add(card);
            }
            // Bottom expensive spells if we're missing lands
            else if (card.getManaValue() >= 4 && handLands < 2) {
                toBottom.add(card);
            } else {
                toTop.add(card);
            }
        }

        // Reconstruct library: top cards, then rest, then bottom cards
        List<Card> newLibrary = new ArrayList<>();
        newLibrary.addAll(toTop);
        newLibrary.addAll(library);
        newLibrary.addAll(toBottom);

        library.clear();
        library.addAll(newLibrary);
    }

    /**
     * Mulligan to a smaller hand size, with scry.
     */
    private static List<Card> mulliganHand(List<Card> library, int handSize, GameRng rng) {
        List<Card> hand = new ArrayList<>(library.subList(0, Math.min(handSize, library.size())));
        library.subList(0, Math.min(handSize, library.size())).clear();

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
     * Uses BO1 hand smoother (draws two hands, chooses better one).
     * @param library The library (will be modified)
     * @param rng Random number generator
     * @return The final hand after all mulligans and scries
     */
    public static List<Card> resolveMulligans(List<Card> library, GameRng rng) {
        // Draw two hands of 7 using BO1 hand smoother
        int drawSize = Math.min(7, library.size());
        List<Card> hand1 = new ArrayList<>(library.subList(0, drawSize));
        library.subList(0, drawSize).clear();

        drawSize = Math.min(7, library.size());
        List<Card> hand2 = new ArrayList<>(library.subList(0, drawSize));
        library.subList(0, drawSize).clear();

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

    // ==================== STARSCOURGE TRIGGER ====================

    /**
     * Resolve Ardyn's Starscourge trigger: exile a creature from graveyard
     * and create a 5/5 Demon token copy.
     */
    private static void resolveStarscourge(GameState state, boolean verbose) {
        Graveyard graveyard = state.getGraveyard();
        List<Card> graveyardCards = graveyard.getCards();

        // Find the best creature in graveyard to exile
        // Priority: high power creatures, especially Bringer/Terror
        int bestIdx = -1;
        int bestPower = 0;

        for (int i = 0; i < graveyardCards.size(); i++) {
            Card card = graveyardCards.get(i);
            if (card instanceof Card.Creature creature) {
                int priorityBoost = 0;
                if (CardNames.is(creature, CardNames.BRINGER_OF_THE_LAST_GIFT)) {
                    priorityBoost = 100;
                } else if (CardNames.is(creature, CardNames.TERROR_OF_THE_PEAKS)) {
                    priorityBoost = 50;
                }
                int effectivePower = creature.getPower() + priorityBoost;

                if (effectivePower > bestPower) {
                    bestPower = effectivePower;
                    bestIdx = i;
                }
            }
        }

        if (bestIdx >= 0) {
            Card creatureCard = graveyard.remove(bestIdx);
            String creatureName = creatureCard.getName();

            if (verbose) {
                System.out.println("[Starscourge] Ardyn exiles " + creatureName + " from graveyard");
            }

            // Add to exile
            state.addToExile(creatureCard);

            // Create a 5/5 Demon token copy of the exiled creature
            Card.Creature tokenCreature = new Card.Creature();
            tokenCreature.setName(creatureName + " (Starscourge Token)");
            tokenCreature.setManaCost(new ManaCost());
            tokenCreature.setManaValue(0);
            tokenCreature.setPower(5);
            tokenCreature.setToughness(5);
            tokenCreature.setLegendary(false);
            tokenCreature.setCreatureTypes(List.of("Demon"));
            tokenCreature.setAbilities(List.of());

            Permanent tokenPerm = new Permanent(tokenCreature, state.getTurn());
            tokenPerm.setIsCopyOf(creatureName);

            // Count Terrors BEFORE adding the token - Terror triggers on "another creature"
            long terrorCount = state.getBattlefield().getPermanents().stream()
                    .filter(p -> CardNames.is(p.getName(), CardNames.TERROR_OF_THE_PEAKS)
                            || CardNames.is(p.getIsCopyOf(), CardNames.TERROR_OF_THE_PEAKS))
                    .count();

            state.getBattlefield().addPermanent(tokenPerm);

            if (verbose) {
                System.out.println("[Starscourge] Created a 5/5 Demon token copy of " + creatureName + " (has haste from Ardyn)");
            }

            // Trigger Terror of the Peaks if on battlefield (for the 5/5 token entering)
            if (terrorCount > 0) {
                int terrorDamage = (int) (5 * terrorCount); // Token is 5/5
                state.setOpponentLife(state.getOpponentLife() - terrorDamage);
                if (verbose) {
                    System.out.println("[Terror] " + terrorDamage + " damage from Starscourge token entering (5 power x " + terrorCount + " Terror(s))");
                }
            }
        }
    }

    // ==================== COMBAT SIMULATION ====================

    /**
     * Simulate combat phase: declare attackers and deal damage.
     * @return Total damage dealt
     */
    public static int simulateCombat(GameState state, boolean verbose) {
        int totalDamage = 0;
        int lifelinkDamage = 0;

        // Check if Ardyn is on the battlefield (for haste and Starscourge)
        boolean ardynOnBattlefield = hasArdynOnBattlefield(state);

        // Resolve Starscourge trigger at beginning of combat (if Ardyn is on battlefield)
        if (ardynOnBattlefield) {
            resolveStarscourge(state, verbose);
        }

        // Find eligible attackers (creatures without summoning sickness, not tapped)
        List<Integer> attackerIndices = new ArrayList<>();
        List<Permanent> permanents = state.getBattlefield().getPermanents();

        for (int idx = 0; idx < permanents.size(); idx++) {
            Permanent permanent = permanents.get(idx);

            // Must be a creature
            if (!(permanent.getCard() instanceof Card.Creature)) {
                continue;
            }

            // Check for impending counters (creature is still an enchantment)
            if (permanent.getCounter(CounterType.TIME) > 0) {
                continue;
            }

            // Check summoning sickness (entered before this turn)
            // Exception: Demons have haste if Ardyn is on battlefield
            boolean hasSummoningSickness = permanent.getTurnEntered() >= state.getTurn();
            if (hasSummoningSickness) {
                boolean demonWithHaste = ardynOnBattlefield && isDemon(permanent);
                if (!demonWithHaste) {
                    continue;
                }
            }

            // Check if tapped
            if (permanent.isTapped()) {
                continue;
            }

            attackerIndices.add(idx);
        }

        // Tap all attackers and calculate damage
        for (int idx : attackerIndices) {
            Permanent permanent = permanents.get(idx);
            boolean isDemonAttacker = ardynOnBattlefield && isDemon(permanent);

            permanent.tap();

            // Get creature power
            if (permanent.getCard() instanceof Card.Creature creature) {
                int power = creature.getPower();
                totalDamage += power;

                // Track lifelink damage for Demons when Ardyn is present
                if (isDemonAttacker) {
                    lifelinkDamage += power;
                }
            }
        }

        // Deal damage to opponent
        state.setOpponentLife(state.getOpponentLife() - totalDamage);

        // Gain life from lifelink
        if (lifelinkDamage > 0) {
            state.setLife(state.getLife() + lifelinkDamage);
            if (verbose) {
                System.out.println("[Combat] Gained " + lifelinkDamage + " life from Demon lifelink");
            }
        }

        if (verbose && totalDamage > 0) {
            System.out.println("[Combat] " + totalDamage + " damage dealt");
        }

        return totalDamage;
    }

    // ==================== TURN EXECUTION ====================

    /**
     * Execute a single turn: untap -> draw -> main -> combat -> end.
     * @return Combat damage dealt
     */
    public static int executeTurn(GameState state, CardDatabase db, boolean verbose, GameRng rng) {
        // Start turn: increment turn counter, untap, reset land drop
        TurnManager.startTurn(state);

        if (verbose) {
            System.out.println("\n=== TURN " + state.getTurn() + " ===");
        }

        // Upkeep phase
        TurnManager.upkeepPhase(state);

        // Draw phase
        state.setPhase(Phase.DRAW);
        int handBefore = state.getHand().size();
        TurnManager.drawPhase(state);

        if (verbose) {
            if (state.getHand().size() > handBefore) {
                List<Card> handCards = state.getHand().getCards();
                if (!handCards.isEmpty()) {
                    Card drawnCard = handCards.get(handCards.size() - 1);
                    System.out.println("[Draw] Drew: " + drawnCard.getName());
                }
            } else if (state.getTurn() == 1 && state.isOnThePlay()) {
                System.out.println("[Draw] Skipped (on the play)");
            }
        }

        // Main phase 1: Play lands and cast spells
        state.setPhase(Phase.MAIN1);

        // Precombat main phase start: advance saga counters and resolve chapters
        TurnManager.precombatMainPhaseStart(state, verbose);

        if (verbose) {
            List<String> handNames = state.getHand().getCards().stream()
                    .map(Card::getName)
                    .toList();
            System.out.println("[Main 1] Hand: " + String.join(", ", handNames));
        }

        executeMainPhase(state, db, verbose, rng);

        // Combat phase
        state.setPhase(Phase.COMBAT);
        int combatDamage = simulateCombat(state, verbose);

        // Main phase 2: Additional spell casting could happen here
        state.setPhase(Phase.MAIN2);
        // For now, we don't do anything in main 2

        // End phase
        state.setPhase(Phase.END);
        TurnManager.endPhase(state);

        if (verbose) {
            System.out.println("[End of Turn " + state.getTurn() + "]");

            List<String> battlefieldNames = state.getBattlefield().getPermanents().stream()
                    .map(p -> {
                        StringBuilder name = new StringBuilder(p.getCard().getName());
                        if (p.getIsCopyOf() != null) {
                            name.append(" (copy of ").append(p.getIsCopyOf()).append(")");
                        }
                        int timeCounters = p.getCounter(CounterType.TIME);
                        if (timeCounters > 0) {
                            name.append(" (").append(timeCounters).append(" time counters)");
                        }
                        return name.toString();
                    })
                    .toList();
            System.out.println("  Battlefield: " + (battlefieldNames.isEmpty() ? "(empty)" : String.join(", ", battlefieldNames)));

            List<String> graveyardNames = state.getGraveyard().getCards().stream()
                    .map(Card::getName)
                    .toList();
            System.out.println("  Graveyard: " + (graveyardNames.isEmpty() ? "(empty)" : String.join(", ", graveyardNames)));

            System.out.println("  Opponent life: " + state.getOpponentLife());
        }

        return combatDamage;
    }

    /**
     * Execute main phase: play lands and cast spells.
     */
    private static void executeMainPhase(GameState state, CardDatabase db, boolean verbose, GameRng rng) {
        mainPhase(state, db, verbose, rng);
    }

    // ==================== MAIN PHASE LOGIC ====================

    /**
     * Core game logic that determines what spells to cast and in what order.
     * Port of TypeScript/Rust mainPhase function.
     */
    public static void mainPhase(GameState state, CardDatabase db, boolean verbose, GameRng rng) {
        List<Card> handCards = state.getHand().getCards();

        // SPECIAL CASE: Turn 4 combo check
        // If we have Spider-Man in hand, and a valid combo target in GY, and can get to 4 mana by playing a land,
        // play the land FIRST before casting any other spells!
        boolean hasSpiderMan = hasCardNamed(handCards, "Superior Spider-Man");
        List<Card> graveyardCards = state.getGraveyard().getCards();
        boolean hasBringerInGy = hasCardNamed(graveyardCards, "Bringer of the Last Gift");

        // Also check for Ardyn combo path
        boolean hasArdynInGy = hasCardNamed(graveyardCards, "Ardyn, the Usurper");
        int otherCreaturesInGy = countCreaturesExcluding(graveyardCards, "Ardyn, the Usurper");
        boolean hasArdynCombo = hasArdynInGy && otherCreaturesInGy >= 1;

        boolean hasValidComboTarget = hasBringerInGy || hasArdynCombo;

        int currentMana = countUntappedLands(state.getBattlefield());

        if (hasSpiderMan && hasValidComboTarget && currentMana == 3 && !state.isLandPlayedThisTurn()) {
            // Check if we have an untapped land to play
            int untappedLandIdx = -1;
            for (int i = 0; i < handCards.size(); i++) {
                Card c = handCards.get(i);
                if (c instanceof Card.Land land) {
                    if (!land.isEntersTapped() && land.getSubtype() != LandSubtype.FASTLAND) {
                        untappedLandIdx = i;
                        break;
                    }
                }
            }

            if (untappedLandIdx >= 0) {
                Card untappedLand = state.getHand().remove(untappedLandIdx);
                String landName = untappedLand.getName();
                CardResolver.playLand(state, untappedLand, verbose);
                if (verbose) {
                    System.out.println("  [COMBO SETUP] Played " + landName + " first to enable turn 4 combo");
                }
            }
        }

        // STEP 1: Cast land-finding spells if we haven't played a land
        // BUT skip if we have Bringer/Terror in hand and can cast discard spell
        boolean hasBringerOrTerrorInHand = hasAnyCardNamed(handCards, "Bringer of the Last Gift", "Terror of the Peaks");

        // Check for Kiora and Speaker in hand in one loop
        boolean kioraInHand = false;
        boolean speakerInHand = false;
        for (int i = 0; i < handCards.size(); i++) {
            String name = handCards.get(i).getName();
            if (name == "Kiora, the Rising Tide") {
                kioraInHand = true;
            } else if (name == "Formidable Speaker") {
                speakerInHand = true;
            }
        }

        boolean shouldPrioritizeDiscardSpell = hasBringerOrTerrorInHand && (kioraInHand || speakerInHand);

        if (!state.isLandPlayedThisTurn() && !shouldPrioritizeDiscardSpell) {
            boolean castAny = true;

            while (castAny && !state.isLandPlayedThisTurn()) {
                castAny = false;

                // Find castable land-finding spells
                int bestFinderIdx = -1;
                int bestManaValue = Integer.MAX_VALUE;

                for (int i = 0; i < state.getHand().size(); i++) {
                    Card c = state.getHand().getCards().get(i);
                    if (LAND_FINDERS.contains(c.getName()) && canCastSpell(c, state)) {
                        if (c.getManaValue() < bestManaValue) {
                            bestManaValue = c.getManaValue();
                            bestFinderIdx = i;
                        }
                    }
                }

                if (bestFinderIdx >= 0) {
                    Card card = state.getHand().remove(bestFinderIdx);
                    String cardName = card.getName();

                    CreatureCard forCreature = (card instanceof Card.Creature creature) ? creature : null;
                    if (tryPayManaCost(CardResolver.getCardManaCost(card), state, forCreature)) {
                        // Handle creatures specially
                        if (card instanceof Card.Creature) {
                            CardResolver.castCreature(state, card, false);
                            // Process ETB triggers for the creature
                            List<Permanent> perms = state.getBattlefield().getPermanents();
                            if (!perms.isEmpty()) {
                                Permanent lastPerm = perms.get(perms.size() - 1);
                                CardResolver.processEtbTriggersVerbose(state, lastPerm, db, verbose, rng);
                            }
                        } else {
                            CardResolver.castSpell(state, card, db, verbose, rng);
                        }

                        if (verbose) {
                            System.out.println("  [Cast] " + cardName);
                        }
                        castAny = true;
                    } else {
                        // Put it back if we can't pay
                        state.getHand().add(card);
                    }
                } else {
                    break;
                }
            }
        }

        // STEP 2: Play a land
        if (!state.isLandPlayedThisTurn()) {
            handCards = state.getHand().getCards();
            boolean hasLands = hasLandsInHand(handCards);

            if (hasLands) {
                OptionalInt landIdxOpt = DecisionEngine.chooseLandToPlay(handCards, state);
                if (landIdxOpt.isPresent()) {
                    Card landCard = state.getHand().remove(landIdxOpt.getAsInt());
                    String cardName = landCard.getName();
                    CardResolver.playLand(state, landCard, verbose);

                    if (verbose) {
                        Permanent lastPerm = null;
                        List<Permanent> perms = state.getBattlefield().getPermanents();
                        if (!perms.isEmpty()) {
                            lastPerm = perms.get(perms.size() - 1);
                        }
                        String tappedStr = (lastPerm != null && lastPerm.isTapped()) ? " (tapped)" : "";
                        System.out.println("  [Land] " + cardName + tappedStr);
                    }
                }
            }
        }

        // STEP 3: Cast remaining spells
        boolean castAny = true;
        while (castAny) {
            castAny = false;

            // Get game state for spell priorities
            List<Card> currentGraveyardCards = state.getGraveyard().getCards();
            List<Card> currentHandCards = state.getHand().getCards();
            boolean hasBringerInGraveyard = hasCardNamed(currentGraveyardCards, "Bringer of the Last Gift");

            // Check for Bringer, Terror, and Spider-Man in hand in one loop
            boolean hasBringerInHandNow = false;
            boolean hasTerrorInHand = false;
            boolean hasSpiderManInHand = false;
            for (int i = 0; i < currentHandCards.size(); i++) {
                String name = currentHandCards.get(i).getName();
                if (name == "Bringer of the Last Gift") {
                    hasBringerInHandNow = true;
                } else if (name == "Terror of the Peaks") {
                    hasTerrorInHand = true;
                } else if (name == "Superior Spider-Man") {
                    hasSpiderManInHand = true;
                }
            }

            // Check if combo would be lethal
            boolean comboIsLethal = hasBringerInGraveyard && CardResolver.isComboLethal(state);

            // Log when holding back combo
            if (verbose && hasBringerInGraveyard && hasSpiderManInHand && !comboIsLethal) {
                int expectedDamage = CardResolver.calculateComboDamage(state);
                System.out.println("  [Waiting] Combo not lethal yet (expected: " + expectedDamage
                        + " damage, need: " + state.getOpponentLife() + ")");
            }

            // Find the best spell to cast
            int bestSpellIdx = -1;
            int bestPriority = Integer.MAX_VALUE;

            for (int i = 0; i < state.getHand().size(); i++) {
                Card c = state.getHand().getCards().get(i);

                // Skip lands
                if (c instanceof Card.Land) {
                    continue;
                }

                // Check if we can cast it
                if (!canCastSpell(c, state)) {
                    continue;
                }

                // Spider-Man casting logic
                if (c.getName() == "Superior Spider-Man") {
                    if (hasBringerInGraveyard) {
                        // Only cast if combo would be lethal
                        if (!comboIsLethal) {
                            continue;
                        }
                    } else {
                        // Check for Ardyn combo path
                        boolean hasArdynInGraveyardNow = hasCardNamed(currentGraveyardCards, "Ardyn, the Usurper");
                        int otherCreatures = countCreaturesExcluding(currentGraveyardCards, "Ardyn, the Usurper");

                        if (!(hasArdynInGraveyardNow && otherCreatures >= 1)) {
                            // No Ardyn combo - check if we should dig
                            int spiderManCount = countCardsNamed(currentHandCards, "Superior Spider-Man");
                            boolean hasMillCreatureInGy = hasMillCreatureInGraveyard(currentGraveyardCards);

                            if (spiderManCount < 2 || !hasMillCreatureInGy) {
                                continue;
                            }
                        }
                    }
                }

                // Calculate priority (lower is better)
                int priority = 1000;
                String cardName = c.getName();

                // Priority 1: Spider-Man if combo is lethal
                if (comboIsLethal && cardName == "Superior Spider-Man") {
                    priority = 1;
                }
                // Priority 1.5: Formidable Speaker if Bringer in GY but no Spider-Man
                else if (hasBringerInGraveyard && !hasSpiderManInHand && cardName == "Formidable Speaker") {
                    priority = 15;
                }
                // Priority 2: Kiora or Speaker if Bringer/Terror in hand
                else if ((hasBringerInHandNow || hasTerrorInHand) && cardName == "Formidable Speaker") {
                    priority = 20;
                } else if ((hasBringerInHandNow || hasTerrorInHand) && cardName == "Kiora, the Rising Tide") {
                    priority = 21;
                }
                // Priority 3: Mill spells
                else if (isMill(cardName)) {
                    priority = 30 + c.getManaValue();
                }
                // Priority 4: Awaken the Honored Dead
                else if (cardName == "Awaken the Honored Dead") {
                    priority = 40;
                }
                // Default: by mana value
                else {
                    priority = 100 + c.getManaValue();
                }

                if (priority < bestPriority) {
                    bestPriority = priority;
                    bestSpellIdx = i;
                }
            }

            if (bestSpellIdx >= 0) {
                Card card = state.getHand().remove(bestSpellIdx);
                String cardName = card.getName();

                // Determine if we should use impending cost
                ManaCost cost;
                boolean useImpending = false;

                if (card instanceof Card.Creature creature && creature.hasImpending()) {
                    ManaCost impendingCost = creature.getImpendingCost();
                    CreatureCard forCreature = creature;
                    if (impendingCost != null && ManaUtils.canPayManaCost(state.getBattlefield(), impendingCost, forCreature, state.getLife())) {
                        useImpending = true;
                        cost = impendingCost;
                    } else {
                        cost = CardResolver.getCardManaCost(card);
                    }
                } else {
                    cost = CardResolver.getCardManaCost(card);
                }

                CreatureCard forCreature = (card instanceof Card.Creature creature) ? creature : null;
                if (tryPayManaCost(cost, state, forCreature)) {
                    if (card instanceof Card.Creature) {
                        CardResolver.castCreature(state, card, useImpending);
                        if (verbose) {
                            if (useImpending) {
                                System.out.println("  [Cast] " + cardName + " (impending)");
                            } else {
                                System.out.println("  [Cast] " + cardName);
                            }
                        }
                        // Process ETB triggers for the creature
                        // Note: ETB triggers fire even when cast for impending (it enters as an enchantment)
                        List<Permanent> perms = state.getBattlefield().getPermanents();
                        if (!perms.isEmpty()) {
                            Permanent lastPerm = perms.get(perms.size() - 1);
                            CardResolver.processEtbTriggersVerbose(state, lastPerm, db, verbose, rng);
                        }
                    } else {
                        CardResolver.castSpell(state, card, db, verbose, rng);
                        if (verbose) {
                            System.out.println("  [Cast] " + cardName);
                        }
                    }

                    castAny = true;
                } else {
                    // Put it back if we can't pay
                    state.getHand().add(card);
                }
            }
        }
    }

    private static boolean isMill(String name) {
        return name.equals("Cache Grab")
                || name.equals("Dredger's Insight")
                || name.equals("Town Greeter")
                || name.equals("Overlord of the Balemurk");
    }

    /**
     * Check if we can cast a spell given current game state.
     * For creatures with impending, also checks if we can afford the impending cost.
     */
    private static boolean canCastSpell(Card card, GameState state) {
        CreatureCard forCreature = (card instanceof Card.Creature creature) ? creature : null;

        // For creatures with impending, check if we can cast for impending cost
        if (forCreature != null && forCreature.hasImpending()) {
            ManaCost impendingCost = forCreature.getImpendingCost();
            if (impendingCost != null && ManaUtils.canPayManaCost(state.getBattlefield(), impendingCost, forCreature, state.getLife())) {
                return true;
            }
        }

        // Check regular mana cost
        ManaCost cost = CardResolver.getCardManaCost(card);
        return ManaUtils.canPayManaCost(state.getBattlefield(), cost, forCreature, state.getLife());
    }

    /**
     * Try to pay a mana cost from the game state's lands.
     * Wrapper around ManaUtils.tryPayManaCost for convenience.
     */
    private static boolean tryPayManaCost(ManaCost cost, GameState state, CreatureCard forCreature) {
        return ManaUtils.tryPayManaCost(state.getBattlefield(), cost, forCreature, state.getLife(), state.getManaPool());
    }

    // ==================== RUN GAME ====================

    /**
     * Run a complete game simulation.
     * @param deck The deck to simulate
     * @param seed Random seed for reproducibility
     * @param db Card database
     * @param verbose Whether to print verbose output
     * @return The game result
     */
    public static GameResult runGame(List<Card> deck, long seed, CardDatabase db, boolean verbose) {
        GameRng rng = new GameRng(seed);

        // Initialize game state
        GameState state = new GameState();

        // Determine if on play or draw (50/50) - BEFORE shuffling to match TypeScript RNG sequence
        state.setOnThePlay(rng.next() < 0.5);

        // Shuffle deck into library
        List<Card> shuffledDeck = new ArrayList<>(deck);
        rng.shuffle(shuffledDeck);
        for (Card card : shuffledDeck) {
            state.getLibrary().addCard(card);
        }

        // Mulligan phase: resolve mulligans to get opening hand
        List<Card> libraryCards = new ArrayList<>();
        while (state.getLibrary().size() > 0) {
            Card card = state.getLibrary().draw();
            if (card != null) {
                libraryCards.add(card);
            }
        }

        List<Card> openingHand = resolveMulligans(libraryCards, rng);

        // Put remaining cards back in library
        for (Card card : libraryCards) {
            state.getLibrary().addCard(card);
        }

        // Add opening hand to hand
        for (Card card : openingHand) {
            state.getHand().add(card);
        }

        // Print game start info if verbose
        if (verbose) {
            System.out.println("=== Game Start (seed: " + seed + ") ===");
            System.out.println(state.isOnThePlay() ? "On the play" : "On the draw");
            System.out.println("Opening hand (" + openingHand.size() + " cards):");
            for (Card card : openingHand) {
                System.out.println("  - " + card.getName());
            }
        }

        // Game loop
        int maxTurns = 20;
        Integer turnWithUbg = null;

        while (state.getTurn() < maxTurns && !checkWinCondition(state)) {
            // Execute turn
            executeTurn(state, db, verbose, rng);

            // Track when all colors become available
            if (turnWithUbg == null) {
                ColorFlags colors = getAvailableColors(state);
                if (colors.hasBlue() && colors.hasBlack() && colors.hasGreen()) {
                    turnWithUbg = state.getTurn();
                }
            }
        }

        return new GameResult(
                checkWinCondition(state) ? state.getTurn() : null,
                turnWithUbg
        );
    }
}
