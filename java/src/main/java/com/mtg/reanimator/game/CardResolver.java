package com.mtg.reanimator.game;

import com.mtg.reanimator.card.*;
import com.mtg.reanimator.game.zones.*;
import com.mtg.reanimator.rng.GameRng;

import java.util.*;

/**
 * Card ability resolution logic.
 * Matches the Rust cards.rs implementation exactly.
 */
public class CardResolver {

    // ==================== HELPER METHODS ====================

    /**
     * Check if a creature has impending counters (enters as enchantment).
     */
    public static boolean hasImpending(Card card) {
        if (card instanceof Card.Creature creature) {
            return creature.hasImpending();
        }
        return false;
    }

    /**
     * Get impending counter count for a creature.
     */
    public static int getImpendingCounters(Card card) {
        if (card instanceof Card.Creature creature) {
            Integer counters = creature.getImpendingCounters();
            return counters != null ? counters : 0;
        }
        return 0;
    }

    /**
     * Get mana cost from any card.
     */
    public static ManaCost getCardManaCost(Card card) {
        return switch (card) {
            case Card.Land land -> new ManaCost(); // Lands have no mana cost
            case Card.Creature creature -> creature.getManaCost();
            case Card.Instant instant -> instant.getManaCost();
            case Card.Sorcery sorcery -> sorcery.getManaCost();
            case Card.Enchantment enchantment -> enchantment.getManaCost();
            case Card.Saga saga -> saga.getManaCost();
        };
    }

    /**
     * Check if Ardyn, the Usurper is on the battlefield.
     */
    public static boolean hasArdynOnBattlefield(GameState state) {
        List<Permanent> permanents = state.getBattlefield().getPermanents();
        for (int i = 0; i < permanents.size(); i++) {
            Permanent p = permanents.get(i);
            if (p.getName() == "Ardyn, the Usurper"
                    || "Ardyn, the Usurper" == p.getIsCopyOf()) {
                return true;
            }
        }
        return false;
    }

    /**
     * Check if a creature card is a Demon.
     */
    public static boolean isCreatureDemon(Card card) {
        if (card instanceof Card.Creature creature) {
            return creature.getCreatureTypes().contains("Demon");
        }
        return false;
    }

    // ==================== PLAY LAND ====================

    /**
     * Play a land from hand to battlefield with proper tapping logic.
     */
    public static void playLand(GameState state, Card card, boolean verbose) {
        if (!(card instanceof Card.Land land)) {
            throw new IllegalArgumentException("Not a land card");
        }

        // Determine if land enters tapped
        boolean entersTapped = land.isEntersTapped();

        // Conditional tapping logic based on land subtype
        LandSubtype subtype = land.getSubtype();
        if (subtype != null) {
            switch (subtype) {
                case SHOCK -> {
                    // Shock lands can pay 2 life to enter untapped
                    // Simplified: always enter untapped
                    entersTapped = false;
                }
                case FASTLAND -> {
                    // Enter untapped if you control 2 or fewer other lands
                    int landCount = 0;
                    List<Permanent> perms = state.getBattlefield().getPermanents();
                    for (int i = 0; i < perms.size(); i++) {
                        if (perms.get(i).getCard() instanceof Card.Land) {
                            landCount++;
                        }
                    }
                    entersTapped = landCount >= 3;
                }
                case TOWN -> {
                    // Starting Town: enters untapped on turns 1-3, tapped on turn 4+
                    entersTapped = state.getTurn() > 3;
                }
                case UTILITY -> {
                    // Verge lands: simplified to always enter untapped
                    boolean isVerge = land.getName().endsWith("Verge");
                    if (isVerge) {
                        entersTapped = false;
                    }
                    // Otherwise use the card's enters_tapped value
                }
                default -> {
                    // Basic, Surveil use enters_tapped from card definition
                }
            }
        }

        Permanent permanent = new Permanent(card, state.getTurn());
        if (entersTapped) {
            permanent.tap();
        }

        // Handle Cavern of Souls - choose creature type
        if (land.getName() == "Cavern of Souls") {
            String chosenType = chooseCavernType(state);
            if (verbose) {
                System.out.println("    (Cavern set to: " + chosenType + ")");
            }
            permanent.setChosenType(chosenType);
        }

        // Handle Multiversal Passage - choose basic land type
        if (land.getName() == "Multiversal Passage") {
            String chosenColor = choosePassageColor(state);
            if (verbose) {
                System.out.println("    (Passage set to: " + chosenColor + ")");
            }
            permanent.setChosenBasicType(chosenColor);
        }

        // Handle surveil lands
        if (land.hasSurveil() && land.getSurveilAmount() > 0) {
            resolveSurveil(state, land.getSurveilAmount(), verbose);
        }

        state.getBattlefield().addPermanent(permanent);
        state.setLandPlayedThisTurn(true);
    }

    // ==================== CHOOSE CAVERN TYPE ====================

    /**
     * Choose creature type for Cavern of Souls.
     * Priority: Human (Spider-Man, Town Greeter) > Demon (Bringer) > Noble (Kiora) > Dragon (Terror) > Avatar (Overlord)
     */
    public static String chooseCavernType(GameState state) {
        // Get creatures in hand
        List<Card> handCards = state.getHand().getCards();
        List<Card> creaturesInHand = new ArrayList<>();
        for (int i = 0; i < handCards.size(); i++) {
            Card c = handCards.get(i);
            if (c instanceof Card.Creature) {
                creaturesInHand.add(c);
            }
        }

        // Check if we already have a Cavern with Human type
        boolean hasHumanCavern = false;
        List<Permanent> perms = state.getBattlefield().getPermanents();
        for (int i = 0; i < perms.size(); i++) {
            Permanent p = perms.get(i);
            if (p.getCard() instanceof Card.Land land) {
                if (land.getName() == "Cavern of Souls"
                        && "Human".equals(p.getChosenType())) {
                    hasHumanCavern = true;
                    break;
                }
            }
        }

        // Check if we have another Cavern in hand
        int cavernsInHand = 0;
        for (int i = 0; i < handCards.size(); i++) {
            if (handCards.get(i).getName() == "Cavern of Souls") {
                cavernsInHand++;
            }
        }

        boolean hasKioraInHand = false;
        boolean hasBringerOrTerrorInHand = false;
        for (int i = 0; i < creaturesInHand.size(); i++) {
            String name = creaturesInHand.get(i).getName();
            if (name == "Kiora, the Rising Tide") {
                hasKioraInHand = true;
            }
            if (name == "Bringer of the Last Gift" || name == "Terror of the Peaks") {
                hasBringerOrTerrorInHand = true;
            }
        }

        // Special case: If we have Kiora + Bringer/Terror in hand AND another Cavern coming,
        // set this one to Noble (cast Kiora first to discard Bringer/Terror)
        if (!hasHumanCavern && hasKioraInHand && hasBringerOrTerrorInHand && cavernsInHand >= 1) {
            return "Noble";
        }

        if (hasHumanCavern) {
            // We already have Human covered, pick something else based on hand
            boolean hasBringer = false;
            boolean hasKiora = false;
            boolean hasOverlord = false;
            boolean hasTerror = false;
            for (int i = 0; i < creaturesInHand.size(); i++) {
                String name = creaturesInHand.get(i).getName();
                if (name == "Bringer of the Last Gift") hasBringer = true;
                if (name == "Kiora, the Rising Tide") hasKiora = true;
                if (name == "Overlord of the Balemurk") hasOverlord = true;
                if (name == "Terror of the Peaks") hasTerror = true;
            }
            if (hasBringer) {
                return "Demon";
            } else if (hasKiora) {
                return "Noble";
            } else if (hasOverlord) {
                return "Avatar";
            } else if (hasTerror) {
                return "Dragon";
            } else {
                // No specific need, default to Demon (in case we draw Bringer)
                return "Demon";
            }
        }

        // First Cavern - default to Human (helps Spider-Man and Town Greeter)
        return "Human";
    }

    // ==================== CHOOSE PASSAGE COLOR ====================

    /**
     * Choose basic land type for Multiversal Passage.
     * Priority: Fill missing colors for castable spells.
     */
    public static String choosePassageColor(GameState state) {
        // Check what colors we currently have access to from untapped lands
        boolean hasBlue = false;
        boolean hasBlack = false;
        boolean hasGreen = false;

        for (Permanent perm : state.getBattlefield().getPermanents()) {
            if (perm.isTapped()) continue;
            if (perm.getCard() instanceof Card.Land land) {
                for (ManaColor color : land.getColors()) {
                    switch (color) {
                        case BLUE -> hasBlue = true;
                        case BLACK -> hasBlack = true;
                        case GREEN -> hasGreen = true;
                        default -> {}
                    }
                }
            }
        }

        // Check what colors we need for spells in hand
        boolean needsBlue = false;
        boolean needsBlack = false;
        boolean needsGreen = false;

        for (Card card : state.getHand().getCards()) {
            ManaCost cost = getCardManaCost(card);
            if (cost.getBlue() > 0) needsBlue = true;
            if (cost.getBlack() > 0) needsBlack = true;
            if (cost.getGreen() > 0) needsGreen = true;
        }

        // Priority: Fill missing colors for castable spells
        if (needsGreen && !hasGreen) {
            return "G";
        } else if (needsBlue && !hasBlue) {
            return "U";
        } else if (needsBlack && !hasBlack) {
            return "B";
        } else if (!hasBlue) {
            // Default: prioritize blue for Spider-Man and Kiora
            return "U";
        } else if (!hasBlack) {
            return "B";
        } else if (!hasGreen) {
            return "G";
        }

        // Fallback
        return "U";
    }

    // ==================== CAST CREATURE ====================

    /**
     * Cast a creature, handling impending logic.
     */
    public static void castCreature(GameState state, Card card, boolean useImpending) {
        if (!(card instanceof Card.Creature)) {
            throw new IllegalArgumentException("Not a creature card");
        }

        Permanent permanent = new Permanent(card, state.getTurn());

        // Handle impending creatures
        if (useImpending && hasImpending(card)) {
            int counters = getImpendingCounters(card);
            permanent.addCounter(CounterType.TIME, counters);
        }

        state.getBattlefield().addPermanent(permanent);
    }

    // ==================== CAST SPELL ====================

    /**
     * Cast a spell and resolve its effects.
     */
    public static void castSpell(GameState state, Card card, CardDatabase db, boolean verbose, GameRng rng) {
        switch (card) {
            case Card.Instant instant -> resolveInstantOrSorcery(state, instant.getAbilities(), card, verbose, rng);
            case Card.Sorcery sorcery -> resolveInstantOrSorcery(state, sorcery.getAbilities(), card, verbose, rng);
            case Card.Enchantment enchantment -> resolveEnchantment(state, enchantment, verbose);
            case Card.Saga saga -> resolveSaga(state, saga, card, verbose);
            default -> throw new IllegalArgumentException("Not a spell card");
        }
    }

    private static void resolveInstantOrSorcery(GameState state, List<String> abilities, Card card, boolean verbose, GameRng rng) {
        for (String ability : abilities) {
            switch (ability) {
                case "mill_4_return_permanent" -> {
                    // Cache Grab: mill 4, return permanent to hand
                    List<Card> milled = state.getLibrary().mill(4);

                    if (verbose) {
                        StringBuilder names = new StringBuilder();
                        for (int i = 0; i < milled.size(); i++) {
                            if (i > 0) names.append(", ");
                            names.append(milled.get(i).getName());
                        }
                        System.out.println("    Mill 4: " + names.toString());
                    }

                    // Filter to permanents only (not instant/sorcery)
                    boolean hasPermanent = false;
                    for (int i = 0; i < milled.size(); i++) {
                        Card c = milled.get(i);
                        if (!(c instanceof Card.Instant || c instanceof Card.Sorcery)) {
                            hasPermanent = true;
                            break;
                        }
                    }

                    // Choose best card to return
                    Card selected = hasPermanent ? selectBestFromMill(milled, state) : null;

                    // Return selected card to hand, rest to graveyard
                    String selectedName = selected != null ? selected.getName() : null;
                    boolean returned = false;
                    for (Card c : milled) {
                        if (!returned && c.getName().equals(selectedName)) {
                            if (verbose) {
                                System.out.println("    -> Returned to hand: " + c.getName());
                            }
                            state.getHand().add(c);
                            returned = true;
                        } else {
                            state.getGraveyard().add(c);
                        }
                    }
                }
                case "search_land_or_creature_with_evidence" -> {
                    // Analyze the Pollen
                    resolveAnalyzeThePollen(state, rng, verbose);
                }
                default -> {}
            }
        }
        // Instant/Sorcery goes to graveyard after resolution
        state.getGraveyard().add(card);
    }

    private static void resolveEnchantment(GameState state, Card.Enchantment enchantment, boolean verbose) {
        // Add enchantment to battlefield
        Permanent permanent = new Permanent(enchantment, state.getTurn());
        state.getBattlefield().addPermanent(permanent);

        // Process enchantment abilities
        for (String ability : enchantment.getAbilities()) {
            if ("etb_mill_4_return_artifact_creature_land".equals(ability)) {
                // Dredger's Insight: mill 4, return artifact/creature/land to hand
                List<Card> milled = state.getLibrary().mill(4);

                if (verbose) {
                    StringBuilder names = new StringBuilder();
                    for (int i = 0; i < milled.size(); i++) {
                        if (i > 0) names.append(", ");
                        names.append(milled.get(i).getName());
                    }
                    System.out.println("    Mill 4: " + names.toString());
                }

                // Choose which card to return (prioritize Spider-Man, then Kiora, then lands)
                int idx = chooseMillReturn(milled, CardType.CREATURE);
                if (idx >= 0) {
                    Card cardToReturn = milled.remove(idx);
                    if (verbose) {
                        System.out.println("    -> Returned to hand: " + cardToReturn.getName());
                    }
                    state.getHand().add(cardToReturn);
                }

                // Rest go to graveyard
                for (Card c : milled) {
                    state.getGraveyard().add(c);
                }
            }
        }
    }

    private static void resolveSaga(GameState state, Card.Saga saga, Card card, boolean verbose) {
        // Add saga to battlefield with 1 lore counter
        String sagaName = saga.getName();
        Permanent permanent = new Permanent(card, state.getTurn());
        permanent.addCounter(CounterType.TIME, 1);
        state.getBattlefield().addPermanent(permanent);

        // Resolve Chapter I immediately
        resolveSagaChapter(state, sagaName, 1, verbose);
    }

    /**
     * Select best card from milled cards to return.
     * NEVER returns Bringer or Terror - they MUST stay in graveyard for reanimation.
     * Priority: Spider-Man > Kiora > lands > non-combo creatures
     */
    private static Card selectBestFromMill(List<Card> milled, GameState state) {
        // COMBO PIECES - NEVER return these, they must stay in graveyard!
        Set<String> comboPieces = Set.of("Bringer of the Last Gift", "Terror of the Peaks");

        // Priority 1: Spider-Man
        for (Card c : milled) {
            if (c.getName() == "Superior Spider-Man") {
                return c;
            }
        }

        // Priority 2: Kiora
        for (Card c : milled) {
            if (c.getName() == "Kiora, the Rising Tide") {
                return c;
            }
        }

        // Priority 3: Any land
        for (Card c : milled) {
            if (c instanceof Card.Land) {
                return c;
            }
        }

        // Priority 4: Any NON-COMBO creature
        for (Card c : milled) {
            if (c instanceof Card.Creature && !comboPieces.contains(c.getName())) {
                return c;
            }
        }

        // Priority 5: Any permanent except combo pieces
        for (Card c : milled) {
            if (!(c instanceof Card.Instant) && !(c instanceof Card.Sorcery)
                    && !comboPieces.contains(c.getName())) {
                return c;
            }
        }

        return null;
    }

    /**
     * Resolve Analyze the Pollen spell.
     * Evidence 8 mechanic - mill then search.
     */
    private static void resolveAnalyzeThePollen(GameState state, GameRng rng, boolean verbose) {
        // Mill 4 as evidence cost (if we have enough in graveyard)
        List<Card> graveyard = state.getGraveyard().getCards();
        int evidenceCount = 0;
        for (Card c : graveyard) {
            evidenceCount += getCardManaValue(c);
            if (evidenceCount >= 8) break;
        }

        // Search for a land or creature
        List<Card> library = new ArrayList<>(state.getLibrary().getCards());
        Card target = null;

        // Priority: Spider-Man > Kiora > any creature > land
        for (Card c : library) {
            if (c.getName() == "Superior Spider-Man") {
                target = c;
                break;
            }
        }
        if (target == null) {
            for (Card c : library) {
                if (c.getName() == "Kiora, the Rising Tide") {
                    target = c;
                    break;
                }
            }
        }
        if (target == null) {
            for (Card c : library) {
                if (c instanceof Card.Creature) {
                    target = c;
                    break;
                }
            }
        }
        if (target == null) {
            for (Card c : library) {
                if (c instanceof Card.Land) {
                    target = c;
                    break;
                }
            }
        }

        if (target != null) {
            findAndRemoveFromLibrary(state.getLibrary(), target.getName());
            state.getHand().add(target);
            if (verbose) {
                System.out.println("    Analyze the Pollen tutors: " + target.getName());
            }
        }

        // Shuffle
        state.getLibrary().shuffle(rng);
    }

    /**
     * Get mana value of a card.
     */
    private static int getCardManaValue(Card card) {
        return card.getManaValue();
    }

    /**
     * Choose which card to return from mill.
     * NEVER returns Bringer or Terror - they MUST stay in graveyard for reanimation.
     * @return index of card to return, or -1 if none
     */
    private static int chooseMillReturn(List<Card> milled, CardType preferredType) {
        // COMBO PIECES - NEVER return these, they must stay in graveyard!
        Set<String> comboPieces = Set.of("Bringer of the Last Gift", "Terror of the Peaks");

        // Priority 1: Spider-Man
        for (int i = 0; i < milled.size(); i++) {
            if (milled.get(i).getName() == "Superior Spider-Man") {
                return i;
            }
        }

        // Priority 2: Kiora
        for (int i = 0; i < milled.size(); i++) {
            if (milled.get(i).getName() == "Kiora, the Rising Tide") {
                return i;
            }
        }

        // Priority 3: Blue-producing lands (critical for combo mana)
        Set<String> blueLands = Set.of("Watery Grave", "Undercity Sewers", "Gloomlake Verge", "Island");
        for (int i = 0; i < milled.size(); i++) {
            Card c = milled.get(i);
            if (c instanceof Card.Land && blueLands.contains(c.getName())) {
                return i;
            }
        }

        // Priority 4: Any land
        for (int i = 0; i < milled.size(); i++) {
            if (milled.get(i) instanceof Card.Land) {
                return i;
            }
        }

        // Priority 5: Non-combo creatures
        for (int i = 0; i < milled.size(); i++) {
            Card c = milled.get(i);
            if (c instanceof Card.Creature && !comboPieces.contains(c.getName())) {
                return i;
            }
        }

        return -1;
    }

    /**
     * Resolve a saga chapter.
     */
    private static void resolveSagaChapter(GameState state, String sagaName, int chapter, boolean verbose) {
        if (sagaName != "Awaken the Honored Dead") {
            return; // Only handle this saga for now
        }

        switch (chapter) {
            case 1 -> {
                // Chapter I: Destroy target permanent (skip in goldfishing)
                if (verbose) {
                    System.out.println("    " + sagaName + " Chapter I: (destroy target - skipped)");
                }
            }
            case 2 -> {
                // Chapter II: Mill 3
                List<Card> milled = state.getLibrary().mill(3);
                for (Card c : milled) {
                    state.getGraveyard().add(c);
                }
                if (verbose) {
                    StringBuilder names = new StringBuilder();
                    for (int i = 0; i < milled.size(); i++) {
                        if (i > 0) names.append(", ");
                        names.append(milled.get(i).getName());
                    }
                    System.out.println("    " + sagaName + " Chapter II: Mill 3 - " + names.toString());
                }
            }
            case 3 -> {
                // Chapter III: Return creature from GY OR search for one
                // Priority: Spider-Man > Kiora > Formidable > Land
                List<Card> graveyard = state.getGraveyard().getCards();
                Card target = null;

                // First check graveyard
                for (Card c : graveyard) {
                    if (c.getName() == "Superior Spider-Man") {
                        target = c;
                        break;
                    }
                }
                if (target == null) {
                    for (Card c : graveyard) {
                        if (c.getName() == "Kiora, the Rising Tide") {
                            target = c;
                            break;
                        }
                    }
                }

                if (target != null) {
                    removeCardFromGraveyard(state.getGraveyard(), target);
                    state.getHand().add(target);
                    if (verbose) {
                        System.out.println("    " + sagaName + " Chapter III: Return from graveyard - " + target.getName());
                    }
                } else if (verbose) {
                    System.out.println("    " + sagaName + " Chapter III: No creature to return");
                }
            }
        }
    }

    /**
     * Helper to find and remove a card from the library by name.
     */
    private static Card findAndRemoveFromLibrary(Library library, String cardName) {
        return library.findAndRemove(cardName);
    }

    /**
     * Helper to remove a specific card from the graveyard.
     */
    private static boolean removeCardFromGraveyard(Graveyard graveyard, Card card) {
        List<Card> cards = graveyard.getCardsMutable();
        return cards.remove(card);
    }

    // ==================== PROCESS ETB TRIGGERS ====================

    /**
     * Process enter-the-battlefield triggers for a creature.
     */
    public static void processEtbTriggersVerbose(GameState state, Permanent permanent, CardDatabase db, boolean verbose, GameRng rng) {
        if (!(permanent.getCard() instanceof Card.Creature creature)) {
            return; // Not a creature
        }

        List<String> abilities = creature.getAbilities();

        for (String ability : abilities) {
            switch (ability) {
                case "etb_mill_4_return_land" -> {
                    // Town Greeter: mill 4, may return land
                    resolveTownGreeterEtb(state, verbose);
                }
                case "etb_draw_2_discard_2" -> {
                    // Kiora: draw 2, discard 2 - use the proper priority logic
                    resolveKioraEtb(state, verbose);
                }
                case "etb_discard_tutor_creature" -> {
                    // Formidable Speaker: may discard a card to tutor a creature
                    resolveFormidableSpeakerEtb(state, rng, verbose);
                }
                case "impending_5" -> {
                    // Impending counters are already added by castCreature when useImpending=true
                    // This ability is just a marker - no action needed here
                }
                case "etb_damage_trigger" -> {
                    // Terror of the Peaks: damage trigger (setup, actual damage on creature ETB)
                    // This is a triggered ability that fires when other creatures enter
                }
                case "etb_mass_reanimate" -> {
                    // Bringer of the Last Gift: mass reanimate
                    List<Card> graveyardCards = new ArrayList<>(state.getGraveyard().getCards());
                    for (Card card : graveyardCards) {
                        if (card instanceof Card.Creature) {
                            Permanent perm = new Permanent(card, state.getTurn());
                            state.getBattlefield().addPermanent(perm);
                        }
                    }
                    state.getGraveyard().clearCreatures();
                }
                case "etb_or_attack_mill_4_return" -> {
                    // Overlord of the Balemurk: mill 4, may return non-Avatar creature or land
                    resolveOverlordEtb(state, verbose);
                }
                case "mind_swap_copy" -> {
                    // Superior Spider-Man: copy creature from graveyard
                    resolveSpiderManCopy(state, permanent, rng, verbose);
                }
                default -> {}
            }
        }
    }

    // ==================== RESOLVE SPIDER-MAN COPY ====================

    /**
     * Superior Spider-Man: copy creature from graveyard.
     * Priority 1: Copy Bringer if in graveyard (THE COMBO!)
     * Priority 2: Copy Ardyn if in graveyard AND there are other creatures
     * Priority 3: If no Bringer/Ardyn but have another Spider-Man in hand, copy a mill creature
     */
    private static void resolveSpiderManCopy(GameState state, Permanent permanent, GameRng rng, boolean verbose) {
        List<Card> graveyardCards = state.getGraveyard().getCards();

        // Priority 1: Bringer
        int bringerIdx = -1;
        for (int i = 0; i < graveyardCards.size(); i++) {
            if (graveyardCards.get(i).getName() == "Bringer of the Last Gift") {
                bringerIdx = i;
                break;
            }
        }

        if (bringerIdx >= 0) {
            if (verbose) {
                System.out.println("    *** COMBO! Superior Spider-Man copies Bringer of the Last Gift! ***");
            }

            permanent.setIsCopyOf("Bringer of the Last Gift");

            // Exile the copied card
            Card bringer = state.getGraveyard().remove(bringerIdx);
            if (bringer != null) {
                state.addToExile(bringer);
            }

            // Trigger Bringer's ETB (mass reanimate!)
            resolveBringerEtb(state, rng, verbose);
            return;
        }

        // Priority 2: Copy Ardyn if in graveyard AND there are other creatures
        int ardynIdx = -1;
        for (int i = 0; i < graveyardCards.size(); i++) {
            if (graveyardCards.get(i).getName() == "Ardyn, the Usurper") {
                ardynIdx = i;
                break;
            }
        }

        int otherCreaturesCount = 0;
        for (int i = 0; i < graveyardCards.size(); i++) {
            Card c = graveyardCards.get(i);
            if (c instanceof Card.Creature && c.getName() != "Ardyn, the Usurper") {
                otherCreaturesCount++;
            }
        }

        if (ardynIdx >= 0 && otherCreaturesCount >= 1) {
            if (verbose) {
                System.out.println("    *** Spider-Man copies Ardyn, the Usurper! (" + otherCreaturesCount + " creatures for Starscourge) ***");
            }

            permanent.setIsCopyOf("Ardyn, the Usurper");

            // Exile Ardyn from graveyard
            Card ardyn = state.getGraveyard().remove(ardynIdx);
            if (ardyn != null) {
                state.addToExile(ardyn);
            }
            return;
        }

        // Priority 3: If no Bringer/Ardyn but have another Spider-Man in hand, copy a mill creature
        int spiderManInHand = 0;
        List<Card> handCards = state.getHand().getCards();
        for (int i = 0; i < handCards.size(); i++) {
            if (handCards.get(i).getName() == "Superior Spider-Man") {
                spiderManInHand++;
            }
        }

        if (spiderManInHand >= 1) {
            // We have another Spider-Man - copy a mill creature to dig for Bringer
            int millCreatureIdx = -1;
            String creatureName = null;

            for (int i = 0; i < graveyardCards.size(); i++) {
                String name = graveyardCards.get(i).getName();
                if (name == "Overlord of the Balemurk") {
                    millCreatureIdx = i;
                    creatureName = name;
                    break;
                }
            }
            if (millCreatureIdx < 0) {
                for (int i = 0; i < graveyardCards.size(); i++) {
                    String name = graveyardCards.get(i).getName();
                    if (name == "Kiora, the Rising Tide") {
                        millCreatureIdx = i;
                        creatureName = name;
                        break;
                    }
                }
            }
            if (millCreatureIdx < 0) {
                for (int i = 0; i < graveyardCards.size(); i++) {
                    String name = graveyardCards.get(i).getName();
                    if (name == "Town Greeter") {
                        millCreatureIdx = i;
                        creatureName = name;
                        break;
                    }
                }
            }

            if (millCreatureIdx >= 0) {
                if (verbose) {
                    System.out.println("    Spider-Man copies " + creatureName + " to dig for Bringer (have another Spider-Man in hand)");
                }

                permanent.setIsCopyOf(creatureName);

                // Exile the copied card
                Card creature = state.getGraveyard().remove(millCreatureIdx);
                if (creature != null) {
                    state.addToExile(creature);
                }

                // Trigger the copied creature's ETB
                switch (creatureName) {
                    case "Overlord of the Balemurk" -> resolveOverlordEtb(state, verbose);
                    case "Kiora, the Rising Tide" -> resolveKioraEtb(state, verbose);
                    case "Town Greeter" -> resolveTownGreeterEtb(state, verbose);
                }
            } else if (verbose) {
                System.out.println("    Spider-Man enters as a 4/4 (no good copy target, but have another Spider-Man)");
            }
        } else if (verbose) {
            System.out.println("    Spider-Man enters as a 4/4 (no good copy target)");
        }
    }

    // ==================== RESOLVE BRINGER ETB ====================

    /**
     * Resolve Bringer of the Last Gift ETB: sacrifice all other creatures, then mass reanimate.
     *
     * EXACT LOGIC FROM RUST:
     * 1. Sacrifice all other creatures (except impending ones with time counters)
     * 2. Return ALL creature cards from graveyard to battlefield
     * 3. Trigger Terror of the Peaks for each creature entering
     */
    public static void resolveBringerEtb(GameState state, GameRng rng, boolean verbose) {
        // Step 1: Sacrifice all other creatures (move to graveyard)
        // NOTE: Impending creatures (with time counters) are NOT creatures yet - they're enchantments!
        // NOTE: The Spider-Man that just entered (copying Bringer) is the last permanent added
        int bringerCopyIdx = Math.max(0, state.getBattlefield().size() - 1);

        List<Integer> toSacrifice = new ArrayList<>();

        List<Permanent> permanents = state.getBattlefield().getPermanents();
        for (int idx = 0; idx < permanents.size(); idx++) {
            Permanent perm = permanents.get(idx);
            // Skip the Spider-Man that just entered (copying Bringer)
            if (idx == bringerCopyIdx) continue;
            // Skip non-creatures
            if (!(perm.getCard() instanceof Card.Creature)) continue;
            // Skip impending creatures (have time counters)
            if (perm.getCounter(CounterType.TIME) > 0) {
                if (verbose) {
                    System.out.println("    Impending survives: " + perm.getName() + " (" + perm.getCounter(CounterType.TIME) + " counters)");
                }
                continue;
            }
            toSacrifice.add(idx);
        }

        if (verbose && !toSacrifice.isEmpty()) {
            StringBuilder names = new StringBuilder();
            for (int i = 0; i < toSacrifice.size(); i++) {
                if (i > 0) names.append(", ");
                names.append(permanents.get(toSacrifice.get(i)).getName());
            }
            System.out.println("    Sacrifice: " + names);
        }

        // Remove sacrificed creatures and add to graveyard (in reverse order to preserve indices)
        for (int i = toSacrifice.size() - 1; i >= 0; i--) {
            Permanent perm = state.getBattlefield().removePermanent(toSacrifice.get(i));
            if (perm != null) {
                state.getGraveyard().add(perm.getCard());
            }
        }

        // Step 2: Return ALL creature cards from graveyard to battlefield
        List<Card> gyCards = state.getGraveyard().getCards();
        List<Card> creaturesToReanimate = new ArrayList<>();
        for (int i = 0; i < gyCards.size(); i++) {
            Card c = gyCards.get(i);
            if (c instanceof Card.Creature) {
                creaturesToReanimate.add(c);
            }
        }

        if (verbose && !creaturesToReanimate.isEmpty()) {
            StringBuilder names = new StringBuilder();
            for (int i = 0; i < creaturesToReanimate.size(); i++) {
                if (i > 0) names.append(", ");
                names.append(creaturesToReanimate.get(i).getName());
            }
            System.out.println("    Reanimate: " + names.toString());
        }

        // Handle Superior Spider-Man's copy choice BEFORE clearing graveyard
        boolean spiderManBeingReanimated = false;
        for (int i = 0; i < creaturesToReanimate.size(); i++) {
            if (creaturesToReanimate.get(i).getName() == "Superior Spider-Man") {
                spiderManBeingReanimated = true;
                break;
            }
        }

        String spiderManCopyTarget = null;
        if (spiderManBeingReanimated) {
            // Look for Terror of the Peaks in graveyard to copy
            boolean terrorInGraveyard = false;
            List<Card> currentGyCards = state.getGraveyard().getCards();
            for (int i = 0; i < currentGyCards.size(); i++) {
                if (currentGyCards.get(i).getName() == "Terror of the Peaks") {
                    terrorInGraveyard = true;
                    break;
                }
            }

            if (terrorInGraveyard) {
                if (verbose) {
                    System.out.println("    Superior Spider-Man (reanimated) copies Terror of the Peaks!");
                }
                // Remove Terror from graveyard and exile it
                List<Card> terrorGyCards = state.getGraveyard().getCards();
                for (int i = 0; i < terrorGyCards.size(); i++) {
                    if (terrorGyCards.get(i).getName() == "Terror of the Peaks") {
                        Card terror = state.getGraveyard().remove(i);
                        if (terror != null) {
                            state.addToExile(terror);
                        }
                        break;
                    }
                }
                spiderManCopyTarget = "Terror of the Peaks";
            } else if (verbose) {
                System.out.println("    Superior Spider-Man (reanimated) enters as a 4/4 (no Terror to copy)");
            }
        }

        // Remove remaining creatures from graveyard
        state.getGraveyard().clearCreatures();

        // Add to battlefield
        for (Card creature : creaturesToReanimate) {
            Permanent perm = new Permanent(creature, state.getTurn());

            // Apply Spider-Man's copy if this is Spider-Man
            if (creature.getName() == "Superior Spider-Man" && spiderManCopyTarget != null) {
                perm.setIsCopyOf(spiderManCopyTarget);
            }

            state.getBattlefield().addPermanent(perm);
        }

        // Step 3: Resolve ETBs for reanimated creatures
        for (Card creature : creaturesToReanimate) {
            switch (creature.getName()) {
                case "Kiora, the Rising Tide" -> resolveKioraEtb(state, verbose);
                case "Town Greeter" -> resolveTownGreeterEtb(state, verbose);
                case "Overlord of the Balemurk" -> resolveOverlordEtb(state, verbose);
                case "Formidable Speaker" -> resolveFormidableSpeakerEtb(state, rng, verbose);
            }
        }

        // Step 4: Resolve Terror triggers for each creature that entered
        resolveTerrorTriggers(state, creaturesToReanimate, verbose);
    }

    // ==================== RESOLVE TERROR TRIGGERS ====================

    /**
     * Resolve Terror of the Peaks triggers for creatures entering the battlefield.
     *
     * EXACT LOGIC:
     * - Count Terrors on battlefield
     * - Each Terror triggers for each OTHER creature entering (not itself)
     * - Deal damage equal to creature's power for each Terror
     */
    public static void resolveTerrorTriggers(GameState state, List<Card> entering, boolean verbose) {
        // Count how many Terrors are on the battlefield
        int terrorCount = 0;
        List<Permanent> perms = state.getBattlefield().getPermanents();
        for (int i = 0; i < perms.size(); i++) {
            Permanent p = perms.get(i);
            if (p.getName() == "Terror of the Peaks"
                    || "Terror of the Peaks" == p.getIsCopyOf()) {
                terrorCount++;
            }
        }

        if (terrorCount == 0) {
            return;
        }

        // Each Terror triggers for each OTHER creature entering
        // (Terror doesn't trigger for itself)
        int totalDamage = 0;

        for (Card creature : entering) {
            if (creature.getName() == "Terror of the Peaks") {
                continue; // Doesn't trigger for itself
            }

            if (creature instanceof Card.Creature c) {
                // Each Terror deals damage equal to the creature's power
                totalDamage += c.getPower() * terrorCount;
            }
        }

        state.setOpponentLife(state.getOpponentLife() - totalDamage);

        if (verbose && totalDamage > 0) {
            System.out.println("  Terror triggers dealt " + totalDamage + " damage! (" + terrorCount + " Terror(s), " + entering.size() + " creatures entered)");
        }
    }

    // ==================== RESOLVE SURVEIL ====================

    /**
     * Resolve surveil mechanic: look at top N cards and decide which go to graveyard.
     *
     * EXACT LOGIC:
     * - Check hasKioraInHand INSIDE the loop (it can change)
     * - Only remove from library if putting in graveyard
     * - If keeping on top, do NOT touch the library - leave card in place
     */
    public static void resolveSurveil(GameState state, int count, boolean verbose) {
        List<String> toGraveyard = new ArrayList<>();
        List<String> toTop = new ArrayList<>();

        for (int i = 0; i < count; i++) {
            // Check if library is empty
            if (state.getLibrary().isEmpty()) {
                break;
            }

            // Peek at top card without removing it
            var topCardOpt = state.getLibrary().peekTop();
            if (topCardOpt.isEmpty()) break;

            Card topCard = topCardOpt.get();
            String cardName = topCard.getName();

            // Decision: keep on top or put in graveyard?
            // Graveyard: Bringer, Terror, Overlord (want to reanimate these)
            // Also put Kiora if we already have one (for reanimation value)
            // Top: Spider-Man (MUST stay in hand!), lands, mill spells
            boolean hasKioraInHand = false;
            List<Card> handCards = state.getHand().getCards();
            for (int j = 0; j < handCards.size(); j++) {
                if (handCards.get(j).getName() == "Kiora, the Rising Tide") {
                    hasKioraInHand = true;
                    break;
                }
            }

            boolean putInGraveyard = cardName == "Bringer of the Last Gift"
                    || cardName == "Terror of the Peaks"
                    || cardName == "Overlord of the Balemurk"
                    || (cardName == "Kiora, the Rising Tide" && hasKioraInHand)
                    || cardName == "Town Greeter"; // Cheap 1/1, better to reanimate than draw

            if (putInGraveyard) {
                // Remove from library and add to graveyard
                Card card = state.getLibrary().draw();
                state.getGraveyard().add(card);
                toGraveyard.add(cardName);
            } else {
                // Keep on top - do NOT touch the library
                toTop.add(cardName);
                break; // Once we keep one on top, we stop (subsequent cards are below it)
            }
        }

        if (verbose && (!toGraveyard.isEmpty() || !toTop.isEmpty())) {
            if (!toGraveyard.isEmpty()) {
                System.out.println("    Surveil -> graveyard: " + String.join(", ", toGraveyard));
            }
            if (!toTop.isEmpty()) {
                System.out.println("    Surveil -> kept on top: " + String.join(", ", toTop));
            }
        }
    }

    // ==================== RESOLVE OVERLORD ETB ====================

    /**
     * Resolve Overlord of the Balemurk ETB ability: mill 4, may return a permanent.
     * Called when Spider-Man copies Overlord to dig for Bringer.
     */
    public static void resolveOverlordEtb(GameState state, boolean verbose) {
        List<Card> milled = state.getLibrary().mill(4);

        if (verbose) {
            StringBuilder millNames = new StringBuilder();
            for (int i = 0; i < milled.size(); i++) {
                if (i > 0) millNames.append(", ");
                millNames.append(milled.get(i).getName());
            }
            System.out.println("    Mill 4: " + millNames.toString());
        }

        // Check game state for selection logic
        boolean hasBringerInGy = false;
        List<Card> gyCards = state.getGraveyard().getCards();
        for (int i = 0; i < gyCards.size(); i++) {
            if (gyCards.get(i).getName() == "Bringer of the Last Gift") {
                hasBringerInGy = true;
                break;
            }
        }
        boolean hasSpiderInHand = false;
        boolean hasBringerInHand = false;
        List<Card> handCards = state.getHand().getCards();
        for (int i = 0; i < handCards.size(); i++) {
            String name = handCards.get(i).getName();
            if (name == "Superior Spider-Man") hasSpiderInHand = true;
            if (name == "Bringer of the Last Gift") hasBringerInHand = true;
        }
        int landCount = 0;
        List<Permanent> perms = state.getBattlefield().getPermanents();
        for (int i = 0; i < perms.size(); i++) {
            if (perms.get(i).getCard() instanceof Card.Land) {
                landCount++;
            }
        }

        Integer selectedIdx = null;

        // Priority 1: Spider-Man if we need it for the combo
        if (hasBringerInGy && !hasSpiderInHand) {
            for (int idx = 0; idx < milled.size(); idx++) {
                if (milled.get(idx).getName() == "Superior Spider-Man") {
                    selectedIdx = idx;
                    if (verbose) {
                        System.out.println("    Overlord returns Superior Spider-Man (combo piece!)");
                    }
                    break;
                }
            }
        }

        // Priority 2: Kiora if Bringer is stuck in hand
        if (selectedIdx == null && hasBringerInHand) {
            for (int idx = 0; idx < milled.size(); idx++) {
                if (milled.get(idx).getName() == "Kiora, the Rising Tide") {
                    selectedIdx = idx;
                    if (verbose) {
                        System.out.println("    Overlord returns Kiora (need to discard Bringer from hand)");
                    }
                    break;
                }
            }
        }

        // Priority 3: Town Greeter if early game
        if (selectedIdx == null && landCount < 4) {
            for (int idx = 0; idx < milled.size(); idx++) {
                if (milled.get(idx).getName() == "Town Greeter") {
                    selectedIdx = idx;
                    if (verbose) {
                        System.out.println("    Overlord returns Town Greeter (cheap enabler)");
                    }
                    break;
                }
            }
        }

        // Otherwise: DON'T return anything! Leave creatures in graveyard for reanimation
        if (selectedIdx == null && verbose) {
            System.out.println("    Overlord returns nothing (keeping creatures for reanimate)");
        }

        // Add cards to graveyard or hand
        for (int idx = 0; idx < milled.size(); idx++) {
            Card card = milled.get(idx);
            if (selectedIdx != null && idx == selectedIdx) {
                state.getHand().add(card);
            } else {
                state.getGraveyard().add(card);
            }
        }
    }

    // ==================== RESOLVE TOWN GREETER ETB ====================

    /**
     * Resolve Town Greeter ETB ability: mill 4, may return a land to hand.
     * Prefer untapped lands > multi-color lands > basic lands
     */
    public static void resolveTownGreeterEtb(GameState state, boolean verbose) {
        List<Card> milled = state.getLibrary().mill(4);

        if (verbose) {
            StringBuilder millNames = new StringBuilder();
            for (int i = 0; i < milled.size(); i++) {
                if (i > 0) millNames.append(", ");
                millNames.append(milled.get(i).getName());
            }
            System.out.println("    Mill 4: " + millNames.toString());
        }

        // Find best land to return
        Integer selectedIdx = null;
        int bestScore = -1;

        for (int idx = 0; idx < milled.size(); idx++) {
            Card card = milled.get(idx);
            if (!(card instanceof Card.Land land)) continue;

            int score = 0;

            // Prefer untapped lands
            if (!land.isEntersTapped()) {
                score += 100;
            }

            // Prefer multi-color lands
            if (land.getColors() != null && land.getColors().size() > 1) {
                score += 50;
            }

            // Prefer lands with surveil
            if (land.hasSurveil()) {
                score += 25;
            }

            // Prefer utility lands (Cavern, Passage)
            if (land.getSubtype() == LandSubtype.UTILITY) {
                score += 75;
            }

            if (score > bestScore) {
                bestScore = score;
                selectedIdx = idx;
            }
        }

        if (selectedIdx != null && verbose) {
            System.out.println("    Town Greeter returns: " + milled.get(selectedIdx).getName());
        } else if (verbose) {
            System.out.println("    Town Greeter returns nothing (no lands milled)");
        }

        // Add cards to graveyard or hand
        for (int idx = 0; idx < milled.size(); idx++) {
            Card card = milled.get(idx);
            if (selectedIdx != null && idx == selectedIdx) {
                state.getHand().add(card);
            } else {
                state.getGraveyard().add(card);
            }
        }
    }

    // ==================== RESOLVE KIORA ETB ====================

    /**
     * Resolve Kiora, the Rising Tide ETB ability: draw 2, discard 2.
     *
     * DISCARD PRIORITY (from Rust):
     * 1. Discard Bringer/Terror/Overlord (we WANT these in graveyard!)
     * 2. Discard excess lands (keep ~4 lands)
     * 3. Discard duplicate creatures (keep 1 of each)
     * 4. Discard lowest priority spells
     * 5. Keep Spider-Man, Kiora, and key enablers
     */
    public static void resolveKioraEtb(GameState state, boolean verbose) {
        // Draw 2
        List<Card> drawn = new ArrayList<>();
        for (int i = 0; i < 2; i++) {
            if (!state.getLibrary().isEmpty()) {
                Card card = state.getLibrary().draw();
                drawn.add(card);
                state.getHand().add(card);
            }
        }

        if (verbose && !drawn.isEmpty()) {
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < drawn.size(); i++) {
                if (i > 0) sb.append(", ");
                sb.append(drawn.get(i).getName());
            }
            System.out.println("    Drew: " + sb);
        }

        // Discard 2
        List<Card> toDiscard = selectKioraDiscards(state, 2);

        for (Card card : toDiscard) {
            state.getHand().remove(card);
            state.getGraveyard().add(card);
        }

        if (verbose && !toDiscard.isEmpty()) {
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < toDiscard.size(); i++) {
                if (i > 0) sb.append(", ");
                sb.append(toDiscard.get(i).getName());
            }
            System.out.println("    Discarded: " + sb);
        }
    }

    /**
     * Select cards to discard for Kiora's ability.
     * Uses 5-priority system from Rust implementation.
     */
    private static List<Card> selectKioraDiscards(GameState state, int count) {
        List<Card> hand = new ArrayList<>(state.getHand().getCards());
        List<Card> toDiscard = new ArrayList<>();

        // Count lands on battlefield
        int landCount = 0;
        List<Permanent> permanents = state.getBattlefield().getPermanents();
        for (int i = 0; i < permanents.size(); i++) {
            if (permanents.get(i).getCard() instanceof Card.Land) {
                landCount++;
            }
        }

        // Check if we already have Bringer in graveyard
        boolean hasBringerInGy = false;
        List<Card> gyCards = state.getGraveyard().getCards();
        for (int i = 0; i < gyCards.size(); i++) {
            if (gyCards.get(i).getName() == "Bringer of the Last Gift") {
                hasBringerInGy = true;
                break;
            }
        }

        while (toDiscard.size() < count && !hand.isEmpty()) {
            int bestIdx = -1;
            int bestPriority = -1;

            for (int i = 0; i < hand.size(); i++) {
                Card card = hand.get(i);
                int priority = getDiscardPriority(card, landCount, hasBringerInGy, hand);

                if (priority > bestPriority) {
                    bestPriority = priority;
                    bestIdx = i;
                }
            }

            if (bestIdx >= 0) {
                Card discarded = hand.remove(bestIdx);
                toDiscard.add(discarded);

                // Update hasBringerInGy if we just discarded Bringer
                if (discarded.getName() == "Bringer of the Last Gift") {
                    hasBringerInGy = true;
                }
            } else {
                break;
            }
        }

        return toDiscard;
    }

    /**
     * Get discard priority for a card (higher = discard first).
     */
    private static int getDiscardPriority(Card card, long landCount, boolean hasBringerInGy, List<Card> hand) {
        String name = card.getName();

        // Priority 1 (highest): Cards we WANT in graveyard
        if (name == "Bringer of the Last Gift") return 500;
        if (name == "Terror of the Peaks") return 490;
        if (name == "Ardyn, the Usurper") return 480;
        if (name == "Overlord of the Balemurk" && hasBringerInGy) return 470;

        // Priority 2: Excess lands (if we have enough)
        if (card instanceof Card.Land) {
            int landsInHand = 0;
            for (int i = 0; i < hand.size(); i++) {
                if (hand.get(i) instanceof Card.Land) landsInHand++;
            }
            if (landCount >= 4 && landsInHand > 1) return 300;
            if (landCount >= 3 && landsInHand > 2) return 250;
        }

        // Priority 3: Duplicate creatures
        if (card instanceof Card.Creature) {
            int copies = 0;
            for (int i = 0; i < hand.size(); i++) {
                if (hand.get(i).getName() == name) copies++;
            }
            if (copies > 1 && name != "Superior Spider-Man") return 200;
        }

        // Priority 4: Low-value spells
        if (card instanceof Card.Instant || card instanceof Card.Sorcery) {
            return 100;
        }

        // Priority 5 (lowest): Keep these!
        if (name == "Superior Spider-Man") return -100;  // NEVER discard - combo piece!
        if (name == "Kiora, the Rising Tide") return -50;

        return 0;
    }

    // ==================== RESOLVE FORMIDABLE SPEAKER ETB ====================

    /**
     * Resolve Formidable Speaker ETB ability: may discard a card, then search for a creature.
     *
     * TUTOR PRIORITY (6 levels from Rust):
     * 1. Bringer (if we have Spider-Man in hand)
     * 2. Spider-Man (if we have Bringer in graveyard)
     * 3. Kiora (draw/discard engine)
     * 4. Overlord (mill/card advantage)
     * 5. Terror (damage combo)
     * 6. Town Greeter (cheap enabler)
     */
    public static void resolveFormidableSpeakerEtb(GameState state, GameRng rng, boolean verbose) {
        List<Card> hand = state.getHand().getCards();
        List<Card> graveyard = state.getGraveyard().getCards();
        List<Card> library = state.getLibrary().getCards();

        // Gather state from hand (single pass)
        boolean hasSpiderMan = false, hasBringerInHand = false, hasTerrorInHand = false;
        boolean hasArdynInHand = false, hasKioraInHand = false, hasOverlordInHand = false;
        int kioraCount = 0, townGreeterCount = 0;
        for (int i = 0; i < hand.size(); i++) {
            String name = hand.get(i).getName();
            if (name == "Superior Spider-Man") hasSpiderMan = true;
            else if (name == "Bringer of the Last Gift") hasBringerInHand = true;
            else if (name == "Terror of the Peaks") hasTerrorInHand = true;
            else if (name == "Ardyn, the Usurper") hasArdynInHand = true;
            else if (name == "Kiora, the Rising Tide") { hasKioraInHand = true; kioraCount++; }
            else if (name == "Overlord of the Balemurk") hasOverlordInHand = true;
            else if (name == "Town Greeter") townGreeterCount++;
        }

        // Gather state from graveyard (single pass)
        boolean hasBringerInGy = false, hasTerrorInGy = false;
        for (int i = 0; i < graveyard.size(); i++) {
            String name = graveyard.get(i).getName();
            if (name == "Bringer of the Last Gift") hasBringerInGy = true;
            else if (name == "Terror of the Peaks") hasTerrorInGy = true;
        }

        // Check library for Spider-Man
        boolean spiderInLibrary = false;
        for (int i = 0; i < library.size(); i++) {
            if (library.get(i).getName() == "Superior Spider-Man") {
                spiderInLibrary = true;
                break;
            }
        }

        String discardTarget = null;
        String tutorTarget = null;

        // Priority 1: Discard Bringer/Terror/Ardyn to get Spider-Man
        if (!hasSpiderMan && spiderInLibrary) {
            if (hasBringerInHand) {
                discardTarget = "Bringer of the Last Gift";
                tutorTarget = "Superior Spider-Man";
            } else if (hasTerrorInHand) {
                discardTarget = "Terror of the Peaks";
                tutorTarget = "Superior Spider-Man";
            } else if (hasArdynInHand) {
                discardTarget = "Ardyn, the Usurper";
                tutorTarget = "Superior Spider-Man";
            }
        }

        // Priority 1.5: If no Spider-Man but Bringer in GY - discard ANY mill creature to get Spider-Man
        if (tutorTarget == null && !hasSpiderMan && hasBringerInGy && spiderInLibrary) {

            if (kioraCount > 1) {
                discardTarget = "Kiora, the Rising Tide";
                tutorTarget = "Superior Spider-Man";
            } else if (townGreeterCount > 1) {
                discardTarget = "Town Greeter";
                tutorTarget = "Superior Spider-Man";
            } else if (kioraCount >= 1) {
                discardTarget = "Kiora, the Rising Tide";
                tutorTarget = "Superior Spider-Man";
            } else if (townGreeterCount >= 1) {
                discardTarget = "Town Greeter";
                tutorTarget = "Superior Spider-Man";
            } else if (hasOverlordInHand) {
                discardTarget = "Overlord of the Balemurk";
                tutorTarget = "Superior Spider-Man";
            }
        }

        // Priority 2: Spider-Man in hand, Bringer not in GY, Bringer in hand -> discard Bringer
        if (tutorTarget == null && hasSpiderMan && !hasBringerInGy && hasBringerInHand) {
            discardTarget = "Bringer of the Last Gift";
            if (!hasTerrorInGy && !hasTerrorInHand) {
                tutorTarget = "Terror of the Peaks";
            } else {
                // Get mill creature instead
                if (!hasOverlordInHand) {
                    tutorTarget = "Overlord of the Balemurk";
                } else if (!hasKioraInHand) {
                    tutorTarget = "Kiora, the Rising Tide";
                } else {
                    tutorTarget = "Superior Spider-Man"; // backup
                }
            }
        }

        // Priority 3: Spider-Man + Bringer in GY but no Terror -> discard land to get Terror
        if (tutorTarget == null && hasSpiderMan && hasBringerInGy && !hasTerrorInGy && !hasTerrorInHand) {
            // Find a land to discard
            boolean hasLandInHand = false;
            for (int i = 0; i < hand.size(); i++) {
                if (hand.get(i) instanceof Card.Land) {
                    hasLandInHand = true;
                    break;
                }
            }
            if (hasLandInHand) {
                discardTarget = "land"; // special marker
                tutorTarget = "Terror of the Peaks";
            }
        }

        // If no valid discard/tutor pair found, decline the ability
        if (tutorTarget == null) {
            if (verbose) {
                System.out.println("    Formidable Speaker ETB: chose not to discard");
            }
            return;
        }

        // Execute discard
        Card toDiscard = null;
        if ("land".equals(discardTarget)) {
            // Find any land
            for (Card card : hand) {
                if (card instanceof Card.Land) {
                    toDiscard = card;
                    break;
                }
            }
        } else {
            // Find card by name
            for (Card card : hand) {
                if (card.getName().equals(discardTarget)) {
                    toDiscard = card;
                    break;
                }
            }
        }

        if (toDiscard == null) {
            if (verbose) {
                System.out.println("    Formidable Speaker ETB: chose not to discard");
            }
            return;
        }

        state.getHand().remove(toDiscard);
        state.getGraveyard().add(toDiscard);

        if (verbose) {
            System.out.println("    Formidable Speaker discards: " + toDiscard.getName());
        }

        // Execute tutor
        Card found = findAndRemoveFromLibrary(state.getLibrary(), tutorTarget);
        if (found != null) {
            state.getHand().add(found);
            if (verbose) {
                System.out.println("    Formidable Speaker tutors: " + found.getName());
            }
        } else if (verbose) {
            System.out.println("    Formidable Speaker: " + tutorTarget + " not in library");
        }

        // Shuffle library
        state.getLibrary().shuffle(rng);
    }

    // ===== COMBO DAMAGE CALCULATION =====

    /**
     * Calculate expected damage from the combo if cast now.
     *
     * Damage sources:
     * 1. Terror triggers from creatures entering (both from battlefield and graveyard)
     * 2. Combat damage from creatures already on battlefield (no summoning sickness)
     * 3. Combat damage from Demons with haste (if Ardyn is on battlefield)
     */
    public static int calculateComboDamage(GameState state) {
        boolean ardynOnBattlefield = hasArdynOnBattlefield(state);

        // Creatures in graveyard that would be reanimated - use indexed loop
        List<Card> gyCards = state.getGraveyard().getCards();
        int creaturesInGraveyardCount = 0;
        int terrorsInGraveyard = 0;
        int creaturePowerSum = 0;
        int reanimatedDemonCombatPower = 0;

        // Single pass through graveyard to collect all needed data
        for (int i = 0; i < gyCards.size(); i++) {
            Card c = gyCards.get(i);
            if (c instanceof Card.Creature creature) {
                creaturesInGraveyardCount++;
                creaturePowerSum += creature.getPower();
                if (c.getName() == CardNames.TERROR_OF_THE_PEAKS) {
                    terrorsInGraveyard++;
                }
                if (ardynOnBattlefield && isCreatureDemon(c)) {
                    reanimatedDemonCombatPower += creature.getPower();
                }
            }
        }

        final int BRINGER_POWER = 6; // Spider-Man copies Bringer

        // Count Terrors on battlefield - use indexed loop
        List<Permanent> permanents = state.getBattlefield().getPermanents();
        int terrorsOnBattlefield = 0;
        int currentCombatPower = 0;
        int currentTurn = state.getTurn();

        for (int i = 0; i < permanents.size(); i++) {
            Permanent p = permanents.get(i);
            // Check for Terror
            if (p.getName() == CardNames.TERROR_OF_THE_PEAKS
                    || CardNames.TERROR_OF_THE_PEAKS.equals(p.getIsCopyOf())) {
                terrorsOnBattlefield++;
            }

            // Check for combat power
            if (p.getCard() instanceof Card.Creature creature) {
                // Check for impending counters
                int timeCounters = p.getCounter(CounterType.TIME);
                if (timeCounters > 0) continue;
                // Check summoning sickness
                boolean hasSummoningSickness = currentTurn <= p.getTurnEntered();
                if (hasSummoningSickness) {
                    // Demons get haste from Ardyn
                    if (ardynOnBattlefield && isCreatureDemon(p.getCard())) {
                        currentCombatPower += creature.getPower();
                    }
                } else {
                    currentCombatPower += creature.getPower();
                }
            }
        }

        int totalTerrors = terrorsOnBattlefield + terrorsInGraveyard;

        // Terror damage calculation
        // Spider-Man (copying Bringer) triggers = 6 * totalTerrors
        int terrorDamage = BRINGER_POWER * totalTerrors;

        // Each creature in graveyard triggers Terror (they enter at the same time)
        // We already computed the sum of creature powers
        terrorDamage += creaturePowerSum * totalTerrors;

        // Terrors trigger on each other (but not themselves)
        if (terrorsInGraveyard > 1) {
            terrorDamage += 3 * terrorsInGraveyard * (terrorsInGraveyard - 1);
        }

        return terrorDamage + currentCombatPower + reanimatedDemonCombatPower;
    }

    /**
     * Check if casting the combo NOW would be lethal.
     */
    public static boolean isComboLethal(GameState state) {
        int expectedDamage = calculateComboDamage(state);
        return expectedDamage >= state.getOpponentLife();
    }
}