use crate::card::Card;
use std::collections::HashMap;

/// Counter types for permanents (e.g., time counters for impending creatures)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterType {
    Time,
}

/// A permanent on the battlefield with state tracking
#[derive(Debug, Clone)]
pub struct Permanent {
    pub card: Card,
    pub tapped: bool,
    pub turn_entered: u32,
    pub counters: HashMap<CounterType, u32>,
    pub chosen_type: Option<String>,      // For Cavern of Souls
    pub chosen_basic_type: Option<String>, // For Multiversal Passage
    pub is_copy_of: Option<String>,       // For Superior Spider-Man
}

impl Permanent {
    pub fn new(card: Card, turn_entered: u32) -> Self {
        Permanent {
            card,
            tapped: false,
            turn_entered,
            counters: HashMap::new(),
            chosen_type: None,
            chosen_basic_type: None,
            is_copy_of: None,
        }
    }

    pub fn with_tapped(mut self, tapped: bool) -> Self {
        self.tapped = tapped;
        self
    }

    pub fn add_counter(&mut self, counter_type: CounterType, amount: u32) {
        *self.counters.entry(counter_type).or_insert(0) += amount;
    }

    pub fn remove_counter(&mut self, counter_type: CounterType, amount: u32) -> bool {
        if let Some(count) = self.counters.get_mut(&counter_type) {
            if *count >= amount {
                *count -= amount;
                if *count == 0 {
                    self.counters.remove(&counter_type);
                }
                return true;
            }
        }
        false
    }

    pub fn get_counter(&self, counter_type: CounterType) -> u32 {
        self.counters.get(&counter_type).copied().unwrap_or(0)
    }
}

/// Library (deck) - ordered stack of cards
#[derive(Debug, Clone)]
pub struct Library {
    cards: Vec<Card>,
}

impl Library {
    pub fn new() -> Self {
        Library { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Add a card to the top of the library (front of the vec)
    pub fn add_to_top(&mut self, card: Card) {
        self.cards.insert(0, card);
    }

    /// Peek at the top card without removing it
    pub fn peek_top(&self) -> Option<&Card> {
        self.cards.first()
    }

    pub fn draw(&mut self) -> Option<Card> {
        if self.cards.is_empty() {
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    pub fn mill(&mut self, count: usize) -> Vec<Card> {
        let mut milled = Vec::new();
        for _ in 0..count {
            if let Some(card) = self.draw() {
                milled.push(card);
            }
        }
        milled
    }

    pub fn size(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.cards.shuffle(&mut rng);
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn cards_mut(&mut self) -> &mut Vec<Card> {
        &mut self.cards
    }
}

/// Hand - cards in hand
#[derive(Debug, Clone)]
pub struct Hand {
    cards: Vec<Card>,
}

impl Hand {
    pub fn new() -> Self {
        Hand { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn remove_card(&mut self, index: usize) -> Option<Card> {
        if index < self.cards.len() {
            Some(self.cards.remove(index))
        } else {
            None
        }
    }

    pub fn find_card(&self, name: &str) -> Option<usize> {
        self.cards.iter().position(|c| c.name() == name)
    }

    pub fn size(&self) -> usize {
        self.cards.len()
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }
}

/// Graveyard - discard pile (ordered stack)
#[derive(Debug, Clone)]
pub struct Graveyard {
    cards: Vec<Card>,
}

impl Graveyard {
    pub fn new() -> Self {
        Graveyard { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn size(&self) -> usize {
        self.cards.len()
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn find_creature(&self) -> Option<Card> {
        self.cards
            .iter()
            .find(|c| matches!(c, Card::Creature(_)))
            .cloned()
    }

    pub fn clear_creatures(&mut self) {
        self.cards.retain(|c| !matches!(c, Card::Creature(_)));
    }

    pub fn remove_card(&mut self, index: usize) -> Option<Card> {
        if index < self.cards.len() {
            Some(self.cards.remove(index))
        } else {
            None
        }
    }
}

/// Battlefield - permanents in play
#[derive(Debug, Clone)]
pub struct Battlefield {
    permanents: Vec<Permanent>,
}

impl Battlefield {
    pub fn new() -> Self {
        Battlefield {
            permanents: Vec::new(),
        }
    }

    pub fn add_permanent(&mut self, permanent: Permanent) {
        self.permanents.push(permanent);
    }

    pub fn remove_permanent(&mut self, index: usize) -> Option<Permanent> {
        if index < self.permanents.len() {
            Some(self.permanents.remove(index))
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        self.permanents.len()
    }

    pub fn permanents(&self) -> &[Permanent] {
        &self.permanents
    }

    pub fn permanents_mut(&mut self) -> &mut [Permanent] {
        &mut self.permanents
    }
}

/// Exile - exiled cards
#[derive(Debug, Clone)]
pub struct Exile {
    cards: Vec<Card>,
}

impl Exile {
    pub fn new() -> Self {
        Exile { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn size(&self) -> usize {
        self.cards.len()
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }
}

