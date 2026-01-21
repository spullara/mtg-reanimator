package com.mtg.reanimator.game;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.game.zones.Battlefield;
import com.mtg.reanimator.game.zones.Exile;
import com.mtg.reanimator.game.zones.Graveyard;
import com.mtg.reanimator.game.zones.Hand;
import com.mtg.reanimator.game.zones.Library;
import com.mtg.reanimator.game.zones.Permanent;
import com.mtg.reanimator.rng.GameRng;

import java.util.List;
import java.util.NoSuchElementException;

/**
 * Complete game state.
 * Matches the Rust GameState struct.
 */
public class GameState {
    // Zones
    private Library library;
    private Hand hand;
    private Graveyard graveyard;
    private Battlefield battlefield;
    private Exile exile;

    // Game info
    private int turn;
    private Phase phase;
    private boolean onThePlay;
    private boolean landPlayedThisTurn;
    private int landPlaysRemaining;

    // Life totals
    private int life;
    private int opponentLife;

    // Win/combo tracking
    private boolean hasWon;
    private int comboDamageDealt;

    // Mana
    private ManaPool manaPool;

    // RNG
    private GameRng rng;

    public GameState() {
        this.library = new Library();
        this.hand = new Hand();
        this.graveyard = new Graveyard();
        this.battlefield = new Battlefield();
        this.exile = new Exile();
        this.turn = 0;
        this.phase = Phase.UNTAP;
        this.onThePlay = false;
        this.landPlayedThisTurn = false;
        this.landPlaysRemaining = 1;
        this.life = 20;
        this.opponentLife = 20;
        this.hasWon = false;
        this.comboDamageDealt = 0;
        this.manaPool = new ManaPool();
        this.rng = new GameRng();
    }

    // ---- Zone accessors ----
    public Library getLibrary() {
        return library;
    }

    public Hand getHand() {
        return hand;
    }

    public Graveyard getGraveyard() {
        return graveyard;
    }

    public Battlefield getBattlefield() {
        return battlefield;
    }

    public Exile getExile() {
        return exile;
    }

    // ---- Game state accessors ----
    public int getTurn() {
        return turn;
    }

    public void setTurn(int turn) {
        this.turn = turn;
    }

    public void incrementTurn() {
        turn++;
    }

    public Phase getPhase() {
        return phase;
    }

    public void setPhase(Phase phase) {
        this.phase = phase;
    }

    public boolean isOnThePlay() {
        return onThePlay;
    }

    public void setOnThePlay(boolean onThePlay) {
        this.onThePlay = onThePlay;
    }

    public boolean isLandPlayedThisTurn() {
        return landPlayedThisTurn;
    }

    public void setLandPlayedThisTurn(boolean landPlayedThisTurn) {
        this.landPlayedThisTurn = landPlayedThisTurn;
    }

    public int getLandPlaysRemaining() {
        return landPlaysRemaining;
    }

    public void setLandPlaysRemaining(int landPlaysRemaining) {
        this.landPlaysRemaining = landPlaysRemaining;
    }

    // ---- Life totals ----
    public int getLife() {
        return life;
    }

    public void setLife(int life) {
        this.life = life;
    }

    public int getOpponentLife() {
        return opponentLife;
    }

    public void setOpponentLife(int opponentLife) {
        this.opponentLife = opponentLife;
    }

    // ---- Win/combo tracking ----
    public boolean hasWon() {
        return hasWon;
    }

    public void setHasWon(boolean hasWon) {
        this.hasWon = hasWon;
    }

    public int getComboDamageDealt() {
        return comboDamageDealt;
    }

    public void setComboDamageDealt(int comboDamageDealt) {
        this.comboDamageDealt = comboDamageDealt;
    }

    // ---- Mana ----
    public ManaPool getManaPool() {
        return manaPool;
    }

    // ---- RNG ----
    public GameRng getRng() {
        return rng;
    }

    public void setRng(GameRng rng) {
        this.rng = rng;
    }

    // ---- Convenience methods ----

    /**
     * Draw a card from the library to hand.
     * @return true if a card was drawn, false if library was empty
     */
    public boolean drawCard() {
        try {
            Card card = library.draw();
            hand.add(card);
            return true;
        } catch (NoSuchElementException e) {
            return false;
        }
    }

    /**
     * Add a card to the graveyard.
     */
    public void addToGraveyard(Card card) {
        graveyard.add(card);
    }

    /**
     * Add a card to exile.
     */
    public void addToExile(Card card) {
        exile.add(card);
    }

    /**
     * Untap all permanents on the battlefield.
     */
    public void untapAll() {
        battlefield.untapAll();
    }

    /**
     * Reset turn state (land plays, mana pool).
     */
    public void resetTurnState() {
        landPlayedThisTurn = false;
        landPlaysRemaining = 1;
        manaPool.clear();
    }

    /**
     * Start a new turn.
     * Increments turn, resets land plays, untaps all permanents, clears mana pool.
     */
    public void startTurn() {
        turn++;
        phase = Phase.UNTAP;
        landPlayedThisTurn = false;
        landPlaysRemaining = 1;
        untapAll();
        // Note: summoning sickness is cleared by checking turnEntered vs current turn
    }

    /**
     * Check if a land can be played.
     */
    public boolean canPlayLand() {
        return landPlaysRemaining > 0 && phase.isMainPhase();
    }

    /**
     * Play a land from hand to battlefield.
     * @param land The land card to play
     * @return true if the land was played successfully
     */
    public boolean playLand(Card land) {
        if (!canPlayLand()) {
            return false;
        }
        // Remove from hand and add to battlefield
        if (hand.remove(land)) {
            battlefield.add(new Permanent(land, turn));
            landPlayedThisTurn = true;
            landPlaysRemaining--;
            return true;
        }
        return false;
    }

    /**
     * Draw multiple cards from the library.
     * @param n Number of cards to draw
     * @return Number of cards actually drawn
     */
    public int drawCards(int n) {
        int drawn = 0;
        for (int i = 0; i < n && !library.isEmpty(); i++) {
            if (drawCard()) {
                drawn++;
            }
        }
        return drawn;
    }

    /**
     * Reset game state for reuse without reallocating.
     */
    public void reset() {
        library = new Library();
        hand = new Hand();
        graveyard = new Graveyard();
        battlefield = new Battlefield();
        exile = new Exile();
        turn = 0;
        phase = Phase.UNTAP;
        onThePlay = false;
        landPlayedThisTurn = false;
        landPlaysRemaining = 1;
        life = 20;
        opponentLife = 20;
        hasWon = false;
        comboDamageDealt = 0;
        manaPool = new ManaPool();
        // Note: rng is not reset - use setRng to provide a new one if needed
    }
}

