package com.mtg.reanimator.game;

import com.mtg.reanimator.card.*;
import com.mtg.reanimator.game.zones.Battlefield;
import com.mtg.reanimator.game.zones.Permanent;

import java.util.*;

import static com.mtg.reanimator.card.CardNames.*;

/**
 * Mana utilities implementing scarcity-based land tapping algorithm.
 * Matches the Rust mana.rs implementation exactly.
 */
public class ManaUtils {

    /**
     * Get the colors a land permanent can produce.
     * Handles basic lands, dual lands, and special lands like Cavern of Souls.
     *
     * @param permanent The permanent to check (must be a land)
     * @param battlefield The battlefield for checking land types (for Verge lands)
     * @param forCreature The creature being cast (for Cavern of Souls), may be null
     * @param life Current life total (for Starting Town)
     * @return ColorFlags of producible colors
     */
    public static ColorFlags getProducedColors(Permanent permanent, Battlefield battlefield,
                                                CreatureCard forCreature, int life) {
        if (permanent.isTapped()) {
            return new ColorFlags();
        }

        Card.Land land = permanent.asLand();
        if (land == null) {
            return new ColorFlags();
        }

        String name = land.getName();

        // Handle Cavern of Souls - colored mana ONLY for creatures of chosen type
        if (name == CAVERN_OF_SOULS) {
            // Cavern always produces colorless
            // Produces any color ONLY for creatures of the chosen type
            if (forCreature != null && permanent.getChosenType() != null) {
                if (creatureMatchesCavernType(forCreature, permanent.getChosenType())) {
                    // Creature matches! Can produce any color
                    return new ColorFlags(ColorFlags.WHITE | ColorFlags.BLUE | ColorFlags.BLACK |
                                          ColorFlags.RED | ColorFlags.GREEN | ColorFlags.COLORLESS);
                }
            }
            // No creature context or creature doesn't match - only colorless
            return new ColorFlags(ColorFlags.COLORLESS);
        }

        // Handle Wastewood Verge - {B} only if controlling Swamp/Forest
        if (name == WASTEWOOD_VERGE) {
            boolean hasSwampOrForest = hasLandNamed(battlefield,
                "Swamp", "Forest", "Watery Grave", "Underground Mortuary", "Undercity Sewers");
            if (hasSwampOrForest) {
                return new ColorFlags(ColorFlags.GREEN | ColorFlags.BLACK);
            }
            return new ColorFlags(ColorFlags.GREEN);
        }

        // Handle Gloomlake Verge - {B} only if controlling Island/Swamp
        if (name == GLOOMLAKE_VERGE) {
            boolean hasIslandOrSwamp = hasLandNamed(battlefield,
                "Island", "Swamp", "Watery Grave", "Undercity Sewers");
            if (hasIslandOrSwamp) {
                return new ColorFlags(ColorFlags.BLUE | ColorFlags.BLACK);
            }
            return new ColorFlags(ColorFlags.BLUE);
        }

        // Handle Multiversal Passage - produces chosen color
        if (name == MULTIVERSAL_PASSAGE) {
            String chosenType = permanent.getChosenBasicType();
            if (chosenType != null) {
                try {
                    ManaColor color = ManaColor.fromString(chosenType);
                    ColorFlags flags = new ColorFlags();
                    flags.insert(color);
                    return flags;
                } catch (IllegalArgumentException e) {
                    // Invalid color string, return empty
                }
            }
            return new ColorFlags();
        }

        // Handle Starting Town - produces C for free, or any color for 1 life
        if (name == STARTING_TOWN) {
            if (life > 1) {
                // Can pay 1 life for any color
                return new ColorFlags(ColorFlags.COLORLESS | ColorFlags.WHITE | ColorFlags.BLUE |
                                      ColorFlags.BLACK | ColorFlags.RED | ColorFlags.GREEN);
            }
            // Only colorless if we can't afford the life
            return new ColorFlags(ColorFlags.COLORLESS);
        }

        // Return land colors for other lands
        ColorFlags flags = new ColorFlags();
        for (ManaColor color : land.getColors()) {
            flags.insert(color);
        }
        return flags;
    }

    /**
     * Check if a creature matches a Cavern of Souls chosen type.
     */
    private static boolean creatureMatchesCavernType(CreatureCard creature, String chosenType) {
        List<String> types = creature.getCreatureTypes();
        for (int i = 0; i < types.size(); i++) {
            if (types.get(i).equalsIgnoreCase(chosenType)) {
                return true;
            }
        }
        return false;
    }

    /**
     * Check if battlefield has a land with any of the given names.
     */
    private static boolean hasLandNamed(Battlefield battlefield, String... names) {
        Set<String> nameSet = Set.of(names);
        for (Permanent p : battlefield.getPermanents()) {
            if (p.isLand() && nameSet.contains(p.getCard().getName())) {
                return true;
            }
        }
        return false;
    }

    /**
     * Check if a mana cost CAN be paid without actually tapping.
     * Uses the same scarcity-based algorithm to ensure consistency.
     */
    public static boolean canPayManaCost(Battlefield battlefield, ManaCost cost,
                                          CreatureCard forCreature, int life) {
        // Collect all land info: (index, colors as flags)
        List<LandInfo> landInfo = collectLandInfo(battlefield, forCreature, life);

        // Quick check: do we have enough total mana?
        int totalCost = cost.getWhite() + cost.getBlue() + cost.getBlack() +
                        cost.getRed() + cost.getGreen() + cost.getColorless() + cost.getGeneric();
        if (landInfo.size() < totalCost) {
            return false;
        }

        // Track which lands are "used" in our simulated assignment
        Set<Integer> usedIndices = new HashSet<>();

        // Build list of (color, amount) pairs, only for colors we need
        List<ColorRequirement> colorsToPayList = buildColorRequirements(cost);

        // Sort colors by scarcity: count how many lands can produce each color
        colorsToPayList.sort(Comparator.comparingInt(req ->
            countLandsProducingColor(landInfo, req.color)));

        // Process colors in order of scarcity
        for (ColorRequirement req : colorsToPayList) {
            int remaining = req.amount;

            // Collect lands that can produce this color, sorted by flexibility
            List<LandCandidate> candidates = new ArrayList<>(landInfo.size());
            for (int i = 0; i < landInfo.size(); i++) {
                LandInfo li = landInfo.get(i);
                if (!usedIndices.contains(li.index) && li.colors.contains(req.color)) {
                    candidates.add(new LandCandidate(li.index, li.colors.count()));
                }
            }
            candidates.sort(Comparator.comparingInt(c -> c.colorCount));

            for (LandCandidate candidate : candidates) {
                if (remaining == 0) break;
                usedIndices.add(candidate.index);
                remaining--;
            }

            if (remaining > 0) {
                return false;
            }
        }

        // Check if we can pay generic with remaining lands
        int genericRemaining = cost.getGeneric();
        int availableForGeneric = 0;
        for (int i = 0; i < landInfo.size(); i++) {
            if (!usedIndices.contains(landInfo.get(i).index)) {
                availableForGeneric++;
            }
        }

        return availableForGeneric >= genericRemaining;
    }

    /**
     * Attempt to pay a mana cost by tapping lands.
     * Uses scarcity-based algorithm: pay rarest colors first using least flexible lands.
     *
     * @return true if cost was paid, false if not enough mana (lands remain untapped on failure)
     */
    public static boolean tryPayManaCost(Battlefield battlefield, ManaCost cost,
                                          CreatureCard forCreature, int life, ManaPool manaPool) {
        // Collect all land info FIRST (before any mutations)
        List<LandInfo> landInfo = collectLandInfo(battlefield, forCreature, life);

        // Quick check: do we have enough total mana?
        int totalCost = cost.getWhite() + cost.getBlue() + cost.getBlack() +
                        cost.getRed() + cost.getGreen() + cost.getColorless() + cost.getGeneric();
        if (landInfo.size() < totalCost) {
            return false;
        }

        // Track which lands to tap (index -> color to produce)
        List<LandToTap> landsToTap = new ArrayList<>();
        Set<Integer> usedIndices = new HashSet<>();

        // Build list of (color, amount) pairs
        List<ColorRequirement> colorsToPayList = buildColorRequirements(cost);

        // Sort colors by scarcity
        colorsToPayList.sort(Comparator.comparingInt(req ->
            countLandsProducingColor(landInfo, req.color)));

        // Process colors in order of scarcity
        for (ColorRequirement req : colorsToPayList) {
            int remaining = req.amount;

            // Get lands that can produce this color, sorted by flexibility (fewer colors = less flexible = use first)
            List<LandCandidate> candidates = new ArrayList<>(landInfo.size());
            for (int i = 0; i < landInfo.size(); i++) {
                LandInfo li = landInfo.get(i);
                if (!usedIndices.contains(li.index) && li.colors.contains(req.color)) {
                    candidates.add(new LandCandidate(li.index, li.colors.count()));
                }
            }
            candidates.sort(Comparator.comparingInt(c -> c.colorCount));

            for (LandCandidate candidate : candidates) {
                if (remaining == 0) break;
                landsToTap.add(new LandToTap(candidate.index, req.color));
                usedIndices.add(candidate.index);
                remaining--;
            }

            if (remaining > 0) {
                return false;
            }
        }

        // Pay generic with remaining untapped lands (prefer least flexible)
        int genericRemaining = cost.getGeneric();
        List<LandInfo> genericCandidates = new ArrayList<>(landInfo.size());
        for (int i = 0; i < landInfo.size(); i++) {
            LandInfo li = landInfo.get(i);
            if (!usedIndices.contains(li.index)) {
                genericCandidates.add(li);
            }
        }
        genericCandidates.sort(Comparator.comparingInt(li -> li.colors.count()));

        for (LandInfo li : genericCandidates) {
            if (genericRemaining == 0) break;
            ManaColor firstColor = li.colors.firstColor();
            if (firstColor != null) {
                landsToTap.add(new LandToTap(li.index, firstColor));
                usedIndices.add(li.index);
                genericRemaining--;
            }
        }

        if (genericRemaining > 0) {
            return false;
        }

        // Now actually tap the lands and add mana to pool
        List<Permanent> permanents = battlefield.getPermanents();
        for (LandToTap ltt : landsToTap) {
            Permanent perm = permanents.get(ltt.index);
            perm.tap();
            manaPool.addMana(ltt.color, 1);
        }

        // Pay the actual cost from the pool
        return manaPool.pay(cost);
    }

    // ========== Helper methods and classes ==========

    /**
     * Collect land information from the battlefield.
     */
    private static List<LandInfo> collectLandInfo(Battlefield battlefield,
                                                   CreatureCard forCreature, int life) {
        List<LandInfo> landInfo = new ArrayList<>();
        List<Permanent> permanents = battlefield.getPermanents();

        for (int i = 0; i < permanents.size(); i++) {
            Permanent p = permanents.get(i);
            if (p.isTapped() || !p.isLand()) {
                continue;
            }
            ColorFlags colors = getProducedColors(p, battlefield, forCreature, life);
            if (!colors.isEmpty()) {
                landInfo.add(new LandInfo(i, colors));
            }
        }
        return landInfo;
    }

    /**
     * Build list of color requirements from a mana cost.
     */
    private static List<ColorRequirement> buildColorRequirements(ManaCost cost) {
        List<ColorRequirement> list = new ArrayList<>();
        if (cost.getWhite() > 0) list.add(new ColorRequirement(ManaColor.WHITE, cost.getWhite()));
        if (cost.getBlue() > 0) list.add(new ColorRequirement(ManaColor.BLUE, cost.getBlue()));
        if (cost.getBlack() > 0) list.add(new ColorRequirement(ManaColor.BLACK, cost.getBlack()));
        if (cost.getRed() > 0) list.add(new ColorRequirement(ManaColor.RED, cost.getRed()));
        if (cost.getGreen() > 0) list.add(new ColorRequirement(ManaColor.GREEN, cost.getGreen()));
        if (cost.getColorless() > 0) list.add(new ColorRequirement(ManaColor.COLORLESS, cost.getColorless()));
        return list;
    }

    /**
     * Count how many lands can produce a given color.
     */
    private static int countLandsProducingColor(List<LandInfo> landInfo, ManaColor color) {
        int count = 0;
        for (int i = 0; i < landInfo.size(); i++) {
            if (landInfo.get(i).colors.contains(color)) {
                count++;
            }
        }
        return count;
    }

    /**
     * Land info record for scarcity calculations.
     */
    private record LandInfo(int index, ColorFlags colors) {}

    /**
     * Color requirement for scarcity sorting.
     */
    private record ColorRequirement(ManaColor color, int amount) {}

    /**
     * Land candidate with flexibility score.
     */
    private record LandCandidate(int index, int colorCount) {}

    /**
     * Land to tap with the color it will produce.
     */
    private record LandToTap(int index, ManaColor color) {}
}
