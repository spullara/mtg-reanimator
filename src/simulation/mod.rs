pub mod deck;
pub mod hand;
pub mod mulligan;
pub mod decisions;
pub mod engine;
pub mod optimize;

pub use deck::{parse_deck_file, DeckError};
pub use hand::select_opening_hand;
pub use mulligan::resolve_mulligans;
pub use decisions::DecisionEngine;
pub use engine::{run_game, execute_turn, check_win_condition, simulate_combat, GameResult};
pub use optimize::{LandConfig, generate_random_land_config_weighted, generate_random_land_config_shuffle, build_deck_from_config, config_to_string};
