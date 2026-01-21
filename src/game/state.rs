use crate::card::Card;
use crate::game::zones::{Battlefield, Exile, Graveyard, Hand, Library};
use crate::game::mana::ManaPool;

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

    /// Add a card to the graveyard
    pub fn add_to_graveyard(&mut self, card: Card) {
        self.graveyard.add_card(card);
    }

    /// Add a card to exile
    pub fn add_to_exile(&mut self, card: Card) {
        self.exile.add_card(card);
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

    /// Reset game state for reuse without reallocating
    pub fn reset(&mut self) {
        self.library = Library::new();  // We'll optimize this more later
        self.hand = Hand::new();
        self.graveyard = Graveyard::new();
        self.battlefield = Battlefield::new();
        self.exile = Exile::new();
        self.turn = 0;
        self.phase = Phase::Untap;
        self.on_the_play = false;
        self.land_played_this_turn = false;
        self.life = 20;
        self.opponent_life = 20;
        self.mana_pool = ManaPool::new();
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}



