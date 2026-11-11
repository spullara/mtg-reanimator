# MTG Reanimator Simulator

A Monte Carlo simulator for Magic: The Gathering reanimator combo decks, written
in Rust, plus a set of MTG Arena tools in [`arena/`](arena/README.md).

The simulator plays thousands of games against an empty board ("goldfish") to
measure how fast a deck wins and why it does not win sooner. It models the
Superior Spider-Man + Bringer of the Last Gift combo and the cards that set it
up.

## Features

- **Fast parallel simulation** with Rayon, and reproducible seeds.
- **Arena-like opening hands:** Bo1 hand smoothing on every 7-card draw
  (of two hands, keep the one closest to the deck's land ratio) and the London
  mulligan (draw 7, put one card on the bottom per mulligan).
- **Play/draw 50/50**, real mana rules (shock lands, Verges, Starting Town,
  Multiversal Passage, slow lands), and land drops planned for the combo turn.
- **Combo gate:** the engine combos when it wins within two turns (combo turn
  plus two attacks). Spider-Man copies are 4/4.
- **Turn 4 report:** how often the combo pieces are ready on turn 4, and how
  often the mana cannot cast them (fewer than 4 mana, or wrong colors).
- **Deck comparison** and a **land optimizer** (pool files, common seeds,
  three-stage search, 23/24/25 land counts).
- **Verbose single games** that print every action.

## Installation

```bash
cargo build --release
```

## Usage

All commands accept `--deck <file>` (default `deck.txt`). The current list is
`deck_latest.txt`.

### Run games

```bash
./target/release/mtg-reanimator run --deck deck_latest.txt --num-games 10000
./target/release/mtg-reanimator run --deck deck_latest.txt --num-games 1 --seed 8 --verbose
```

With `--seed S`, game *i* uses seed *S + i*. Seeds 1, 2 and 3 therefore share
almost all of their games. For independent runs use seed bases that are far
apart, for example `1000001`, `2000001`, `3000001`.

### Turn 4 analysis

```bash
./target/release/mtg-reanimator analyze --deck deck_latest.txt --num-games 50000 --seed 1000001
```

Prints why the combo did not fire on turn 4 (pieces first, then mana), the
combo-pieces-on-turn-4 report, wrong mana, and color-locked spells.

### Compare decks

```bash
./target/release/mtg-reanimator compare deck_a.txt deck_b.txt --num-games 10000
```

### Optimize lands

```bash
./target/release/mtg-reanimator optimize \
  --deck deck_latest.txt --pool pools/latest_ub.txt --seed 1 \
  --land-counts 24,25 --spell Deceit \
  --out deck_latest_opt24.txt,deck_latest_opt25.txt
```

- `--pool` — land pool file, one line per land: `<min> <max> <Land name>`.
  Use `min = max` to fix a land (for example Cavern of Souls).
- `--land-counts` — 23, 24 and/or 25. 25 cuts one spell, 23 adds one.
- `--spell` — limit which spells 23/25 may cut or add (repeat the flag).
- Stages: random configurations, the top 10 again with 50,000 games, then
  single-land swaps until no swap helps by 0.01 turns.

### Mana production

```bash
./target/release/mtg-reanimator mana --deck deck_mana_test.txt --num-games 10000 --turns 10
```

## Deck file format

One card per line, `COUNT CARD NAME`. Arena export lines also work: the set
code and collector number are ignored, and `Deck` / `Sideboard` headers are
skipped.

```
Deck
4 Dark Matter Manipulator (FRA) 52
4 Superior Spider-Man (SPM) 155
2 Terror of the Peaks (M21) 164
4 Bringer of the Last Gift (LCI) 94
4 Watery Grave (EOE) 261
```

## Card database

Cards are defined in `cards.json` (types, costs, colors, abilities). Card
behavior that the engine models is in `src/game/cards.rs` and
`src/simulation/`. A card that is not in `cards.json` stops the deck parser
with "Card not found".

The model is a goldfish: removal, counterspells, hand attack and blockers do
nothing, and lands do not cost life.

## Example output

```
Win rate: 100.0% (9999/10000)
Average win turn: 6.44
Wrong mana on turn 4: 0.0% of games (0.0% of games with 4+ mana)
First turn able to pay {2}{U}{B}: 4.44 on average (95.7% of games)

Combo pieces ready on turn 4: 49.6% of games
  castable:      70.0% of ready (34.8% of games)
  NOT castable:  30.0% of ready (14.9% of games)
    fewer than 4 mana:  28.3% of ready
    wrong colors:        1.7% of ready

Turn distribution:
  Turn  4:   5.6% ██ (561)
  Turn  5:  25.9% ████████████ (2594)
  Turn  6:  28.0% ██████████████ (2803)
  Turn  7:  19.2% █████████ (1924)
```

## MTG Arena tools

[`arena/`](arena/README.md) has Python tools that read MTG Arena's
`Player.log` (with Detailed Logs on): a play-by-play parser, a live dashboard,
a move advisor (Claude, `claude-opus-5-5`, with an offline fallback) that reads
the deck from the match and the strategy from `arena/strategies/`, and a game
archive with a strategy report. They run on Windows and macOS. See
[arena/README.md](arena/README.md) for setup.

## License

MIT
