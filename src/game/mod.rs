pub mod mana;
pub mod state;
pub mod zones;
pub mod turns;
pub mod cards;

pub use mana::ManaPool;
pub use state::{GameState, Phase};
pub use zones::{Battlefield, Exile, Graveyard, Hand, Library, Permanent};
pub use turns::{start_turn, draw_phase, upkeep_phase, end_phase, can_attack, can_play_land};
pub use cards::{can_cast, play_land, tap_land_for_mana, process_etb_triggers, cast_spell, advance_saga};
