use serde::{Deserialize, Serialize};

/// Mana colors in Magic: The Gathering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManaColor {
    #[serde(rename = "W")]
    White,
    #[serde(rename = "U")]
    Blue,
    #[serde(rename = "B")]
    Black,
    #[serde(rename = "R")]
    Red,
    #[serde(rename = "G")]
    Green,
    #[serde(rename = "C")]
    Colorless,
}

impl ManaColor {
    /// Convert to the single character representation
    pub fn to_char(&self) -> char {
        match self {
            ManaColor::White => 'W',
            ManaColor::Blue => 'U',
            ManaColor::Black => 'B',
            ManaColor::Red => 'R',
            ManaColor::Green => 'G',
            ManaColor::Colorless => 'C',
        }
    }
}

/// Mana cost for a card
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManaCost {
    #[serde(default)]
    pub white: u32,
    #[serde(default)]
    pub blue: u32,
    #[serde(default)]
    pub black: u32,
    #[serde(default)]
    pub red: u32,
    #[serde(default)]
    pub green: u32,
    #[serde(default)]
    pub colorless: u32,
    #[serde(default)]
    pub generic: u32,
}

impl ManaCost {
    pub fn total_value(&self) -> u32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless + self.generic
    }
}

/// Card types in Magic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardType {
    Land,
    Creature,
    Instant,
    Sorcery,
    Enchantment,
    Saga,
}

/// Land subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LandSubtype {
    Basic,
    Shock,
    Surveil,
    Utility,
    Fastland,
    Town,
}

/// Base card properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseCard {
    pub name: String,
    #[serde(default)]
    pub mana_cost: ManaCost,
    pub mana_value: u32,
}

/// Land card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandCard {
    #[serde(flatten)]
    pub base: BaseCard,
    pub subtype: LandSubtype,
    pub enters_tapped: bool,
    pub colors: Vec<ManaColor>,
    #[serde(default)]
    pub has_surveil: bool,
    #[serde(default)]
    pub surveil_amount: u32,
}

/// Creature card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureCard {
    #[serde(flatten)]
    pub base: BaseCard,
    pub power: u32,
    pub toughness: u32,
    #[serde(default)]
    pub is_legendary: bool,
    pub creature_types: Vec<String>,
    pub abilities: Vec<String>,
    #[serde(default)]
    pub impending_cost: Option<ManaCost>,
    #[serde(default)]
    pub impending_counters: Option<u32>,
}

/// Spell card (Instant, Sorcery, Enchantment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCard {
    #[serde(flatten)]
    pub base: BaseCard,
    pub abilities: Vec<String>,
}

/// Saga card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaCard {
    #[serde(flatten)]
    pub base: BaseCard,
    pub chapters: Vec<String>,
}

/// Unified card enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "card_type", rename_all = "lowercase")]
pub enum Card {
    Land(LandCard),
    Creature(CreatureCard),
    Instant(SpellCard),
    Sorcery(SpellCard),
    Enchantment(SpellCard),
    Saga(SagaCard),
}

impl Card {
    pub fn name(&self) -> &str {
        match self {
            Card::Land(c) => &c.base.name,
            Card::Creature(c) => &c.base.name,
            Card::Instant(c) => &c.base.name,
            Card::Sorcery(c) => &c.base.name,
            Card::Enchantment(c) => &c.base.name,
            Card::Saga(c) => &c.base.name,
        }
    }

    pub fn mana_value(&self) -> u32 {
        match self {
            Card::Land(c) => c.base.mana_value,
            Card::Creature(c) => c.base.mana_value,
            Card::Instant(c) => c.base.mana_value,
            Card::Sorcery(c) => c.base.mana_value,
            Card::Enchantment(c) => c.base.mana_value,
            Card::Saga(c) => c.base.mana_value,
        }
    }
}

