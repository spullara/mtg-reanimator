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

    /// Convert to a bit flag
    #[inline]
    pub const fn to_flag(&self) -> u8 {
        match self {
            ManaColor::White => ColorFlags::WHITE,
            ManaColor::Blue => ColorFlags::BLUE,
            ManaColor::Black => ColorFlags::BLACK,
            ManaColor::Red => ColorFlags::RED,
            ManaColor::Green => ColorFlags::GREEN,
            ManaColor::Colorless => ColorFlags::COLORLESS,
        }
    }
}

/// Bitflag representation of mana colors for fast operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColorFlags(pub u8);

impl ColorFlags {
    pub const WHITE: u8 = 1 << 0;
    pub const BLUE: u8 = 1 << 1;
    pub const BLACK: u8 = 1 << 2;
    pub const RED: u8 = 1 << 3;
    pub const GREEN: u8 = 1 << 4;
    pub const COLORLESS: u8 = 1 << 5;

    #[inline]
    pub const fn new() -> Self {
        ColorFlags(0)
    }

    #[inline]
    pub fn insert(&mut self, color: ManaColor) {
        self.0 |= color.to_flag();
    }

    #[inline]
    pub const fn contains_flag(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    #[inline]
    pub const fn has_white(&self) -> bool {
        self.contains_flag(Self::WHITE)
    }

    #[inline]
    pub const fn has_blue(&self) -> bool {
        self.contains_flag(Self::BLUE)
    }

    #[inline]
    pub const fn has_black(&self) -> bool {
        self.contains_flag(Self::BLACK)
    }

    #[inline]
    pub const fn has_red(&self) -> bool {
        self.contains_flag(Self::RED)
    }

    #[inline]
    pub const fn has_green(&self) -> bool {
        self.contains_flag(Self::GREEN)
    }

    #[inline]
    pub const fn has_colorless(&self) -> bool {
        self.contains_flag(Self::COLORLESS)
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Count how many colors are set
    #[inline]
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Check if exactly one color is set
    #[inline]
    pub const fn is_single_color(&self) -> bool {
        self.count() == 1
    }

    /// Check if a specific ManaColor is present
    #[inline]
    pub fn contains(&self, color: ManaColor) -> bool {
        self.contains_flag(color.to_flag())
    }

    /// Get the first (any) color that's set, for generic mana payment
    #[inline]
    pub fn first_color(&self) -> Option<ManaColor> {
        if self.has_white() { return Some(ManaColor::White); }
        if self.has_blue() { return Some(ManaColor::Blue); }
        if self.has_black() { return Some(ManaColor::Black); }
        if self.has_red() { return Some(ManaColor::Red); }
        if self.has_green() { return Some(ManaColor::Green); }
        if self.has_colorless() { return Some(ManaColor::Colorless); }
        None
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

