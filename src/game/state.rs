use crate::card::Card;
use crate::game::zones::{Battlefield, Exile, Graveyard, Hand, Library, Permanent};
use crate::game::mana::ManaPool;
use std::collections::HashMap;

/// Game phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Untap,
    Draw,
    Main1,
    Combat,
    Main2,
    End,
}

impl Phase {
    pub fn next(&self) -> Phase {
        match self {
            Phase::Untap => Phase::Draw,
            Phase::Draw => Phase::Main1,
            Phase::Main1 => Phase::Combat,
            Phase::Combat => Phase::Main2,
            Phase::Main2 => Phase::End,
            Phase::End => Phase::Untap,
        }
    }
}

/// Complete game state
#[derive(Debug, Clone)]
pub struct GameState {
    // Zones
    pub library: Library,
    pub hand: Hand,
    pub graveyard: Graveyard,
    pub battlefield: Battlefield,
    pub exile: Exile,

    // Game info
    pub turn: u32,
    pub phase: Phase,
    pub on_the_play: bool,
    pub land_played_this_turn: bool,

    // Life totals
    pub life: i32,
    pub opponent_life: i32,

    // Mana
    pub mana_pool: ManaPool,

    // Saga tracking (card name -> lore counter count)
    pub saga_counters: HashMap<String, u32>,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            library: Library::new(),
            hand: Hand::new(),
            graveyard: Graveyard::new(),
            battlefield: Battlefield::new(),
            exile: Exile::new(),
            turn: 0,
            phase: Phase::Untap,
            on_the_play: false,
            land_played_this_turn: false,
            life: 20,
            opponent_life: 20,
            mana_pool: ManaPool::new(),
            saga_counters: HashMap::new(),
        }
    }

    /// Draw a card from the library to hand
    pub fn draw_card(&mut self) -> bool {
        if let Some(card) = self.library.draw() {
            self.hand.add_card(card);
            true
        } else {
            false
        }
    }

    /// Play a land from hand to battlefield
    pub fn play_land(&mut self, card_index: usize) -> bool {
        if let Some(card) = self.hand.remove_card(card_index) {
            let permanent = Permanent::new(card, self.turn);
            self.battlefield.add_permanent(permanent);
            self.land_played_this_turn = true;
            true
        } else {
            false
        }
    }

    /// Add a card to the graveyard
    pub fn add_to_graveyard(&mut self, card: Card) {
        self.graveyard.add_card(card);
    }

    /// Add a card to exile
    pub fn add_to_exile(&mut self, card: Card) {
        self.exile.add_card(card);
    }

    /// Cast a spell from hand
    pub fn cast_spell(&mut self, card_index: usize) -> bool {
        if let Some(card) = self.hand.remove_card(card_index) {
            // Spell goes to graveyard after resolution
            self.graveyard.add_card(card);
            true
        } else {
            false
        }
    }

    /// Enter a creature from hand to battlefield
    pub fn enter_creature(&mut self, card_index: usize) -> bool {
        if let Some(card) = self.hand.remove_card(card_index) {
            let permanent = Permanent::new(card, self.turn);
            self.battlefield.add_permanent(permanent);
            true
        } else {
            false
        }
    }

    /// Untap all permanents
    pub fn untap_all(&mut self) {
        for permanent in self.battlefield.permanents_mut() {
            permanent.tapped = false;
        }
    }

    /// Reset turn state
    pub fn reset_turn_state(&mut self) {
        self.land_played_this_turn = false;
        self.mana_pool.clear();
    }

    /// Advance to next phase
    pub fn next_phase(&mut self) {
        self.phase = self.phase.next();
    }

    /// Check if player has won
    pub fn has_won(&self) -> bool {
        self.opponent_life <= 0
    }

    /// Check if player has lost
    pub fn has_lost(&self) -> bool {
        self.life <= 0
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{BaseCard, CreatureCard};

    #[test]
    fn test_game_state_creation() {
        let state = GameState::new();
        assert_eq!(state.life, 20);
        assert_eq!(state.opponent_life, 20);
        assert_eq!(state.turn, 0);
        assert_eq!(state.phase, Phase::Untap);
    }

    #[test]
    fn test_draw_card() {
        let mut state = GameState::new();
        let card = Card::Creature(CreatureCard {
            base: BaseCard {
                name: "Test Creature".to_string(),
                mana_cost: Default::default(),
                mana_value: 1,
            },
            power: 1,
            toughness: 1,
            is_legendary: false,
            creature_types: vec![],
            abilities: vec![],
            impending_cost: None,
            impending_counters: None,
        });

        state.library.add_card(card.clone());
        assert!(state.draw_card());
        assert_eq!(state.hand.size(), 1);
    }

    #[test]
    fn test_phase_progression() {
        let mut state = GameState::new();
        assert_eq!(state.phase, Phase::Untap);
        state.next_phase();
        assert_eq!(state.phase, Phase::Draw);
        state.next_phase();
        assert_eq!(state.phase, Phase::Main1);
    }

    #[test]
    fn test_win_condition() {
        let mut state = GameState::new();
        assert!(!state.has_won());
        state.opponent_life = 0;
        assert!(state.has_won());
    }
}

