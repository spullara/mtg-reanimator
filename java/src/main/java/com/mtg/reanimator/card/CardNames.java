package com.mtg.reanimator.card;

/**
 * Interned card name constants for identity comparison.
 * Using == instead of .equals() is ~5x faster for string comparison.
 */
public final class CardNames {
    private CardNames() {}
    
    // Key combo pieces
    public static final String BRINGER_OF_THE_LAST_GIFT = "Bringer of the Last Gift".intern();
    public static final String TERROR_OF_THE_PEAKS = "Terror of the Peaks".intern();
    public static final String SUPERIOR_SPIDER_MAN = "Superior Spider-Man".intern();
    
    // Mill/Draw creatures
    public static final String KIORA_THE_RISING_TIDE = "Kiora, the Rising Tide".intern();
    public static final String OVERLORD_OF_THE_BALEMURK = "Overlord of the Balemurk".intern();
    public static final String TOWN_GREETER = "Town Greeter".intern();
    public static final String FORMIDABLE_SPEAKER = "Formidable Speaker".intern();
    
    // Special creatures
    public static final String ARDYN_THE_USURPER = "Ardyn, the Usurper".intern();
    
    // Sagas
    public static final String AWAKEN_THE_HONORED_DEAD = "Awaken the Honored Dead".intern();
    
    // Lands
    public static final String STARTING_TOWN = "Starting Town".intern();
    public static final String CAVERN_OF_SOULS = "Cavern of Souls".intern();
    public static final String MULTIVERSAL_PASSAGE = "Multiversal Passage".intern();
    public static final String GLOOMLAKE_VERGE = "Gloomlake Verge".intern();
    public static final String WASTEWOOD_VERGE = "Wastewood Verge".intern();
    public static final String UNDERCITY_SEWERS = "Undercity Sewers".intern();
    
    // Spells
    public static final String DREDGERS_INSIGHT = "Dredger's Insight".intern();
    public static final String CACHE_GRAB = "Cache Grab".intern();
    
    /**
     * Check if card has this name using identity comparison.
     * Since card names are interned at load time, == works.
     */
    public static boolean is(Card card, String internedName) {
        return card.getName() == internedName;
    }
    
    /**
     * Check if card has this name using identity comparison.
     */
    public static boolean is(String cardName, String internedName) {
        return cardName == internedName;
    }
}

