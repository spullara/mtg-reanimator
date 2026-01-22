package com.mtg.reanimator.game;

import com.mtg.reanimator.card.Card;
import com.mtg.reanimator.card.SagaCard;
import com.mtg.reanimator.game.zones.Battlefield;
import com.mtg.reanimator.game.zones.CounterType;
import com.mtg.reanimator.game.zones.Graveyard;
import com.mtg.reanimator.game.zones.Hand;
import com.mtg.reanimator.game.zones.Permanent;

import java.util.ArrayList;
import java.util.List;

/**
 * Manages turn structure and beginning-of-turn effects.
 * Matches the Rust implementation in src/game/turns.rs.
 */
public final class TurnManager {

    private TurnManager() {
        // Utility class - prevent instantiation
    }

    /**
     * Start a new turn: increment turn counter, untap all permanents, reset land drop.
     * Called at the start of each turn.
     *
     * @param state The current game state
     */
    public static void startTurn(GameState state) {
        state.incrementTurn();
        state.resetTurnState();
        state.untapAll();
    }

    /**
     * Draw phase: draw 1 card (skip on turn 1 if on the play).
     *
     * @param state The current game state
     */
    public static void drawPhase(GameState state) {
        // Skip draw on turn 1 if on the play
        if (state.getTurn() == 1 && state.isOnThePlay()) {
            return;
        }
        state.drawCard();
    }

    /**
     * Upkeep phase: trigger upkeep effects.
     * Saga advancement happens in precombatMainPhaseStart (per MTG rules).
     *
     * @param state The current game state
     */
    public static void upkeepPhase(GameState state) {
        // Upkeep effects would be triggered here
        // Saga advancement happens in precombatMainPhaseStart (per MTG rules)
    }

    /**
     * Precombat main phase start: advance saga counters and resolve chapters.
     * According to MTG rules, saga lore counters are added at the beginning of
     * the precombat main phase.
     *
     * @param state   The current game state
     * @param verbose Whether to print verbose output
     */
    public static void precombatMainPhaseStart(GameState state, boolean verbose) {
        Battlefield battlefield = state.getBattlefield();
        List<Permanent> permanents = battlefield.getPermanentsMutable();

        // First pass: collect saga info (indices, names, max chapters) without modifying
        // Only advance sagas that were cast before this turn
        List<SagaInfo> sagasToAdvance = new ArrayList<>();

        for (int i = 0; i < permanents.size(); i++) {
            Permanent permanent = permanents.get(i);
            Card card = permanent.getCard();

            if (card instanceof Card.Saga saga) {
                // Only advance if saga was cast before this turn
                if (permanent.getTurnEntered() < state.getTurn()) {
                    sagasToAdvance.add(new SagaInfo(i, saga.getName(), saga.getChapters().size()));
                }
            }
        }

        // Second pass: advance counters and collect chapters to resolve
        // Note: We use TIME counters for both sagas (lore) and impending creatures
        // to match CardResolver's implementation
        List<SagaChapter> sagaChapters = new ArrayList<>();

        for (SagaInfo info : sagasToAdvance) {
            Permanent permanent = permanents.get(info.index);
            permanent.addCounter(CounterType.TIME, 1);
            int chapter = permanent.getCounter(CounterType.TIME);
            sagaChapters.add(new SagaChapter(info.name, chapter));
        }

        // Third pass: resolve chapters
        for (SagaChapter sc : sagaChapters) {
            CardActions.resolveSagaChapter(state, sc.name, sc.chapter, verbose);
        }

        // Fourth pass: remove completed sagas (put in graveyard)
        List<Integer> indicesToRemove = new ArrayList<>();
        for (SagaInfo info : sagasToAdvance) {
            Permanent permanent = permanents.get(info.index);
            int counters = permanent.getCounter(CounterType.TIME);
            if (counters >= info.maxChapters) {
                indicesToRemove.add(info.index);
            }
        }

        // Remove in reverse order to preserve indices, putting sagas in graveyard
        for (int i = indicesToRemove.size() - 1; i >= 0; i--) {
            int idx = indicesToRemove.get(i);
            Permanent removed = battlefield.remove(idx);
            if (removed != null) {
                state.getGraveyard().add(removed.getCard());
            }
        }
    }

    /**
     * End phase: decrement time counters on impending creatures, discard to 7.
     * Note: Sagas also use time counters but they count UP, not down - don't touch them!
     *
     * @param state The current game state
     */
    public static void endPhase(GameState state) {
        Battlefield battlefield = state.getBattlefield();
        List<Permanent> permanents = battlefield.getPermanentsMutable();

        // Decrement time counters on impending creatures only
        // Sagas are NOT creatures - they use Card.Saga variant
        for (Permanent permanent : permanents) {
            Card card = permanent.getCard();
            // Only decrement counters on creatures (impending creatures use time counters)
            if (card instanceof Card.Creature) {
                int timeCounters = permanent.getCounter(CounterType.TIME);
                if (timeCounters > 0) {
                    permanent.removeCounter(CounterType.TIME, 1);
                }
            }
        }

        // Discard to hand size 7 if needed
        Hand hand = state.getHand();
        Graveyard graveyard = state.getGraveyard();
        while (hand.size() > 7) {
            // In a full implementation, this would choose which card to discard
            // For now, just remove the last card
            Card discarded = hand.remove(hand.size() - 1);
            if (discarded != null) {
                graveyard.add(discarded);
            }
        }
    }

    /**
     * Check if the game should end.
     * Win condition: Deal 20+ combo damage (opponent life <= 0).
     *
     * @param state The current game state
     * @return true if the win condition has been met
     */
    public static boolean checkWinCondition(GameState state) {
        return state.getOpponentLife() <= 0;
    }

    // Helper record for saga info collection
    private record SagaInfo(int index, String name, int maxChapters) {}

    // Helper record for saga chapter resolution
    private record SagaChapter(String name, int chapter) {}
}

