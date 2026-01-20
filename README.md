# MTG Reanimator Simulator

A Monte Carlo simulator for Magic: The Gathering reanimator combo decks, written in Rust. This tool helps analyze and optimize deck configurations by simulating thousands of games to calculate win rates and turn distributions.

## Features

- **Fast parallel simulation** - Uses Rayon for multi-threaded game simulation
- **Verbose game tracing** - Step-by-step output showing every game action
- **Deck comparison** - Compare win rates between two deck configurations
- **Land optimization** - Automatically search for optimal land configurations
- **Reproducible results** - Seed-based RNG for reproducible simulations

## Installation

```bash
cargo build --release
```

## Usage

### Run Simulation (Default)

Run 1000 games with random seeds:
```bash
./target/release/mtg-reanimator
```

Run a single verbose game to see the full play-by-play:
```bash
./target/release/mtg-reanimator -v
```

Run with a specific seed for reproducibility:
```bash
./target/release/mtg-reanimator --seed 12345
```

Use a different deck file:
```bash
./target/release/mtg-reanimator --deck my-deck.txt
```

### Run Command

Explicitly run simulations with more options:
```bash
./target/release/mtg-reanimator run --num-games 5000 --deck deck.txt
```

### Compare Decks

Compare win rates between two deck configurations:
```bash
./target/release/mtg-reanimator compare deck1.txt deck2.txt --num-games 1000
```

### Optimize Lands

Search for optimal land configurations:
```bash
./target/release/mtg-reanimator optimize --configs 100 --games 1000 --strategy weighted
```

Strategies:
- `weighted` - Generate configurations with weighted random selection
- `shuffle` - Generate configurations by shuffling land slots

## Deck File Format

Deck files are plain text with one card per line in the format `COUNT CARD_NAME`:

```
4 Terror of the Peaks
4 Bringer of the Last Gift
4 Superior Spider-Man
4 Kiora, the Rising Tide
4 Overlord of the Balemurk
4 Dredger's Insight
4 Awaken the Honored Dead
4 Cache Grab
3 Town Greeter
3 Watery Grave
3 Undercity Sewers
3 Underground Mortuary
4 Cavern of Souls
2 Multiversal Passage
3 Swamp
2 Island
2 Forest
1 Restless Cottage
1 Wastewood Verge
1 Analyze the Pollen
```

## Card Database

Cards are defined in `cards.json`. The simulator supports:
- Basic and dual lands (shock lands, surveil lands, etc.)
- Creatures with ETB triggers
- Spells with mill, surveil, and reanimation effects
- The Bringer of the Last Gift combo

## Example Output

```
=== MTG Reanimator Simulator ===

Deck: deck.txt (60 cards)
Games: 1000

=== Results ===

Win rate: 87.3% (873/1000)
Average win turn: 5.42
Average UBG available: turn 2.15

Turn distribution:
  Turn  4: 12.5% █████████████ (125)
  Turn  5: 38.2% ██████████████████████████████████████ (382)
  Turn  6: 28.1% ████████████████████████████ (281)
  Turn  7:  8.5% █████████ (85)
```

## How It Works

The simulator models the reanimator combo deck strategy:

1. **Setup phase** - Play lands, use mill/surveil effects to fill the graveyard with creatures
2. **Combo turn** - Cast Superior Spider-Man copying Bringer of the Last Gift to reanimate the entire graveyard
3. **Win condition** - Terror of the Peaks triggers deal lethal damage when creatures enter

The AI makes decisions about:
- Mulligan evaluation
- Land sequencing for color availability
- When to cast setup spells vs. hold for combo
- Optimal timing for the combo turn

## License

MIT

