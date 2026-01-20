/**
 * MTG Reanimator Combo Deck Simulator
 *
 * CARD DESCRIPTIONS:
 * ==================
 *
 * LANDS:
 * ------
 * Forest - Basic Land — Forest ({T}: Add {G}.)
 *
 * Island - Basic Land — Island ({T}: Add {U}.)
 *
 * Swamp - Basic Land — Swamp ({T}: Add {B}.)
 *
 * Watery Grave - Land — Island Swamp ({T}: Add {U} or {B}.)
 *   As this land enters, you may pay 2 life. If you don't, it enters tapped.
 *
 * Cavern of Souls - Land
 *   As this land enters, choose a creature type.
 *   {T}: Add {C}.
 *   {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type,
 *   and that spell can't be countered.
 *
 * Restless Cottage - Land
 *   This land enters tapped.
 *   {T}: Add {B} or {G}.
 *   {2}{B}{G}: This land becomes a 4/4 black and green Horror creature until end of turn. It's still a land.
 *   Whenever this land attacks, create a Food token and exile up to one target card from a graveyard.
 *
 * Undercity Sewers - Land — Island Swamp ({T}: Add {U} or {B}.)
 *   This land enters tapped.
 *   When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
 *
 * Underground Mortuary - Land — Swamp Forest ({T}: Add {B} or {G}.)
 *   This land enters tapped.
 *   When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
 *
 * Wastewood Verge - Land
 *   {T}: Add {G}.
 *   {T}: Add {B}. Activate only if you control a Swamp or a Forest.
 *
 * Gloomlake Verge - Land
 *   {T}: Add {U}.
 *   {T}: Add {B}. Activate only if you control an Island or a Swamp.
 *
 * Multiversal Passage - Land
 *   As this land enters, choose a basic land type. Then you may pay 2 life. If you don't, it enters tapped.
 *   This land is the chosen type.
 *
 * Blooming Marsh - Land ({T}: Add {B} or {G}.)
 *   This land enters tapped unless you control two or fewer other lands.
 *
 * Starting Town - Land — Town
 *   This land enters tapped unless it's your first, second, or third turn of the game.
 *   {T}: Add {C}.
 *   {T}, Pay 1 life: Add one mana of any color.
 *
 * CREATURES:
 * ----------
 * Terror of the Peaks - {3}{R}{R} Creature — Dragon 5/4
 *   Flying
 *   Spells your opponents cast that target this creature cost an additional 3 life to cast.
 *   Whenever another creature you control enters, this creature deals damage equal to that creature's power to any target.
 *
 * Bringer of the Last Gift - {6}{B}{B} Creature — Vampire Demon 6/6
 *   Flying
 *   When this creature enters, if you cast it, each player sacrifices all other creatures they control.
 *   Then each player returns all creature cards from their graveyard that weren't put there this way to the battlefield.
 *
 * Overlord of the Balemurk - {3}{B}{B} Enchantment Creature — Avatar Horror 5/5
 *   Impending 5—{1}{B} (If you cast this spell for its impending cost, it enters with five time counters
 *   and isn't a creature until the last is removed. At the beginning of your end step, remove a time counter from it.)
 *   Whenever this permanent enters or attacks, mill four cards, then you may return a non-Avatar creature card
 *   or a planeswalker card from your graveyard to your hand.
 *
 * Kiora, the Rising Tide - {2}{U} Legendary Creature — Merfolk Noble 3/2
 *   When Kiora enters, draw two cards, then discard two cards.
 *   Threshold — Whenever Kiora attacks, if there are seven or more cards in your graveyard,
 *   you may create Scion of the Deep, a legendary 8/8 blue Octopus creature token.
 *
 * Town Greeter - {1}{G} Creature — Human Citizen 1/1
 *   When this creature enters, mill four cards. You may put a land card from among them into your hand.
 *   If you put a Town card into your hand this way, you gain 2 life.
 *
 * Superior Spider-Man / Kavaero, Mind-Bitten - {2}{U}{B} Legendary Creature — Spider Human Hero 4/4
 *   (Paper name: Kavaero, Mind-Bitten. Arena uses Superior Spider-Man due to Marvel crossover.)
 *   Mind Swap — You may have this creature enter as a copy of any creature card in a graveyard,
 *   except his name is Superior Spider-Man and he's a 4/4 Spider Human Hero in addition to his other types.
 *   When you do, exile that card.
 *
 * SPELLS:
 * -------
 * Analyze the Pollen - {G} Sorcery
 *   As an additional cost to cast this spell, you may collect evidence 8.
 *   (Exile cards with total mana value 8 or greater from your graveyard.)
 *   Search your library for a basic land card. If evidence was collected,
 *   instead search your library for a creature or land card. Reveal that card,
 *   put it into your hand, then shuffle.
 *
 * Cache Grab - {1}{G} Instant
 *   Mill four cards. You may put a permanent card from among the cards milled this way into your hand.
 *   If you control a Squirrel or returned a Squirrel card to your hand this way, create a Food token.
 *
 * Dredger's Insight - {1}{G} Enchantment
 *   Whenever one or more artifact and/or creature cards leave your graveyard, you gain 1 life.
 *   When this enchantment enters, mill four cards. You may put an artifact, creature, or land card
 *   from among the milled cards into your hand.
 *
 * Awaken the Honored Dead - {B}{G}{U} Enchantment — Saga
 *   (As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
 *   I — Destroy target nonland permanent.
 *   II — Mill three cards.
 *   III — You may discard a card. When you do, return target creature or land card from your graveyard to your hand.
 */

// ============================================================================
// PHASE 1: CORE DATA STRUCTURES
// ============================================================================
// SEEDED RANDOM NUMBER GENERATOR
// ============================================================================

// Mulberry32 - a simple, fast 32-bit seeded PRNG
let currentSeed: number;

function mulberry32(seed: number): () => number {
  return function() {
    let t = seed += 0x6D2B79F5;
    t = Math.imul(t ^ t >>> 15, t | 1);
    t ^= t + Math.imul(t ^ t >>> 7, t | 61);
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  };
}

let random: () => number;

function setSeed(seed: number): void {
  currentSeed = seed;
  random = mulberry32(seed);
}

function getSeed(): number {
  return currentSeed;
}

// Parse command line arguments
// Usage: bun run simulator.ts [seed] [deckfile]
// --- Help ---
function printHelp() {
  console.log(`
MTG Reanimator Simulator
========================

USAGE:
  bun run simulator.ts [options]

MODES:

  1. Single Game (verbose) + Statistics:
     bun run simulator.ts [seed] [deck.txt]

     Examples:
       bun run simulator.ts                    # Random seed, deck.txt
       bun run simulator.ts 12345              # Seed 12345, deck.txt
       bun run simulator.ts 12345 deck2.txt    # Seed 12345, deck2.txt
       bun run simulator.ts random deck2.txt   # Random seed, deck2.txt

  2. Compare Two Decks:
     bun run simulator.ts compare <deck1.txt> <deck2.txt> [numGames]

     Examples:
       bun run simulator.ts compare deck.txt deck-aggro.txt
       bun run simulator.ts compare deck.txt deck-control.txt 5000

  3. Optimize Lands:
     bun run simulator.ts optimize [numConfigs] [gamesPerConfig] [strategy]

     Randomly generates land configurations and finds the best one.
     Keeps non-land cards fixed, only changes land counts.

     Strategy options:
       weighted (default) - Random counts for each land type
       shuffle            - Put max copies of each land in pool, shuffle, take 24

     Examples:
       bun run simulator.ts optimize                     # 1000 configs, 1000 games each (weighted)
       bun run simulator.ts optimize 500 2000            # 500 configs, 2000 games each (weighted)
       bun run simulator.ts optimize 1000 1000 shuffle   # Use shuffle strategy

OPTIONS:
  -h, --help    Show this help message
  [seed]        Integer seed for reproducible games, or "random"
  [deck.txt]    Path to deck file (default: deck.txt)
  [numGames]    Number of games for comparison mode (default: 1000)

DECK FILE FORMAT:
  # Comments start with # or //
  4 Terror of the Peaks
  4 Bringer of the Last Gift
  2 Forest
  ...

OUTPUT:
  - Single mode: One verbose game showing every action, then statistics
  - Compare mode: Side-by-side comparison of win rates and turn distributions
`);
}

// Check for help flag
if (process.argv.includes("-h") || process.argv.includes("--help")) {
  printHelp();
  process.exit(0);
}

// Examples:
//   bun run simulator.ts                    # Random seed, deck.txt
//   bun run simulator.ts 12345              # Seed 12345, deck.txt
//   bun run simulator.ts 12345 deck2.txt    # Seed 12345, deck2.txt
//   bun run simulator.ts random deck2.txt   # Random seed, deck2.txt
const argSeed = process.argv[2] && process.argv[2] !== "random" && process.argv[2] !== "compare" ? parseInt(process.argv[2], 10) : null;
const argDeckFile = process.argv[3] ?? "deck.txt";
const initialSeed = argSeed ?? Math.floor(Math.random() * 2147483647);
setSeed(initialSeed);

// Global deck file to use
let DECK_FILE = argDeckFile;

// ============================================================================

// --- Mana Colors ---
type ManaColor = "W" | "U" | "B" | "R" | "G" | "C";

interface ManaCost {
  W?: number;
  U?: number;
  B?: number;
  R?: number;
  G?: number;
  C?: number; // Colorless
  generic?: number; // Can be paid with any color
}

interface ManaPool {
  W: number;
  U: number;
  B: number;
  R: number;
  G: number;
  C: number;
}

// --- Card Types ---
type CardType = "land" | "creature" | "instant" | "sorcery" | "enchantment" | "saga";
type LandSubtype = "basic" | "shock" | "surveil" | "utility" | "fastland" | "town";

interface BaseCard {
  name: string;
  type: CardType;
  manaCost?: ManaCost;
  manaValue: number; // Total mana value for evidence calculation
}

interface LandCard extends BaseCard {
  type: "land";
  subtype: LandSubtype;
  entersTapped: boolean | "conditional"; // conditional = shock/passage lands
  colors: ManaColor[]; // Colors it can produce
  hasSurveil?: boolean;
  surveilAmount?: number;
}

interface CreatureCard extends BaseCard {
  type: "creature";
  power: number;
  toughness: number;
  isLegendary?: boolean;
  creatureTypes: string[];
  abilities: string[]; // Reference to ability handlers
  impendingCost?: ManaCost; // For Overlord of the Balemurk
  impendingCounters?: number; // Number of time counters when cast for impending
}

interface SpellCard extends BaseCard {
  type: "instant" | "sorcery" | "enchantment";
  abilities: string[];
}

interface SagaCard extends BaseCard {
  type: "saga";
  chapters: string[]; // Ability for each chapter
}

type Card = LandCard | CreatureCard | SpellCard | SagaCard;

// --- Battlefield Permanents ---
interface BattlefieldPermanent {
  card: Card;
  tapped: boolean;
  turnEntered: number; // For summoning sickness
  counters?: { time?: number }; // For impending
  isCopyOf?: string; // For Superior Spider-Man copying
  chosenType?: string; // For Cavern of Souls
  chosenBasicType?: ManaColor; // For Multiversal Passage
}

// --- Game State ---
interface GameState {
  // Zones
  library: Card[];
  hand: Card[];
  graveyard: Card[];
  battlefield: BattlefieldPermanent[];
  exile: Card[];

  // Game info
  turn: number;
  phase: "untap" | "draw" | "main1" | "combat" | "main2" | "end";
  onThePlay: boolean;
  landPlayedThisTurn: boolean;

  // Life totals
  life: number;
  opponentLife: number;

  // Mana
  manaPool: ManaPool;

  // Saga tracking
  sagaCounters: Map<string, number>; // card instance id -> lore counters
}

// ============================================================================
// PHASE 1: CARD DATABASE (loaded from cards.json)
// ============================================================================

import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

// JSON card format (snake_case from cards.json)
interface JsonCard {
  name: string;
  card_type: string;
  mana_value: number;
  subtype?: string;
  enters_tapped?: boolean;
  colors?: string[];
  has_surveil?: boolean;
  surveil_amount?: number;
  mana_cost?: Record<string, number>;
  power?: number;
  toughness?: number;
  creature_types?: string[];
  abilities?: string[];
  is_legendary?: boolean;
  impending_cost?: Record<string, number>;
  impending_counters?: number;
  chapters?: string[];
}

// Convert color names from JSON format to ManaColor
function convertColorName(color: string): ManaColor {
  const colorMap: Record<string, ManaColor> = {
    white: "W",
    blue: "U",
    black: "B",
    red: "R",
    green: "G",
    colorless: "C",
    // Also handle already-converted format
    W: "W",
    U: "U",
    B: "B",
    R: "R",
    G: "G",
    C: "C",
  };
  return colorMap[color] || (color as ManaColor);
}

// Convert mana cost from JSON format to ManaCost
function convertManaCost(jsonCost: Record<string, number> | undefined): ManaCost | undefined {
  if (!jsonCost) return undefined;
  const result: ManaCost = {};
  for (const [key, value] of Object.entries(jsonCost)) {
    if (key === "generic") {
      result.generic = value;
    } else {
      const color = convertColorName(key);
      result[color] = value;
    }
  }
  return result;
}

// Determine entersTapped value based on subtype
function determineEntersTapped(
  subtype: string | undefined,
  entersTapped: boolean | undefined
): boolean | "conditional" {
  // Shock, fastland, and town lands have conditional entry
  if (subtype === "shock" || subtype === "fastland" || subtype === "town") {
    return "conditional";
  }
  // Multiversal Passage is utility but still conditional (pay 2 life)
  // We'll handle this by name check
  return entersTapped ?? false;
}

// Convert a single JSON card to the TypeScript Card type
function convertJsonCard(json: JsonCard): Card {
  const baseCard = {
    name: json.name,
    manaValue: json.mana_value,
  };

  if (json.card_type === "land") {
    let entersTapped = determineEntersTapped(json.subtype, json.enters_tapped);
    // Special case for Multiversal Passage - it's utility but conditional
    if (json.name === "Multiversal Passage") {
      entersTapped = "conditional";
    }

    const landCard: LandCard = {
      ...baseCard,
      type: "land",
      subtype: (json.subtype || "basic") as LandSubtype,
      entersTapped,
      colors: (json.colors || []).map(convertColorName),
    };
    if (json.has_surveil) landCard.hasSurveil = true;
    if (json.surveil_amount) landCard.surveilAmount = json.surveil_amount;
    return landCard;
  }

  if (json.card_type === "creature") {
    const creatureCard: CreatureCard = {
      ...baseCard,
      type: "creature",
      manaCost: convertManaCost(json.mana_cost),
      power: json.power || 0,
      toughness: json.toughness || 0,
      creatureTypes: json.creature_types || [],
      abilities: json.abilities || [],
    };
    if (json.is_legendary) creatureCard.isLegendary = true;
    if (json.impending_cost) creatureCard.impendingCost = convertManaCost(json.impending_cost);
    if (json.impending_counters) creatureCard.impendingCounters = json.impending_counters;
    return creatureCard;
  }

  if (json.card_type === "saga") {
    return {
      ...baseCard,
      type: "saga",
      manaCost: convertManaCost(json.mana_cost),
      chapters: json.chapters || [],
    } as SagaCard;
  }

  // instant, sorcery, enchantment
  return {
    ...baseCard,
    type: json.card_type as CardType,
    manaCost: convertManaCost(json.mana_cost),
    abilities: json.abilities || [],
  } as SpellCard;
}

// Load and convert cards from JSON file
function loadCardDatabase(): Record<string, Card> {
  // Get the directory of the current module
  const __filename = fileURLToPath(import.meta.url);
  const __dirname = dirname(__filename);
  const cardsPath = join(__dirname, "cards.json");

  const jsonContent = readFileSync(cardsPath, "utf-8");
  const jsonCards: JsonCard[] = JSON.parse(jsonContent);

  const database: Record<string, Card> = {};
  for (const jsonCard of jsonCards) {
    const card = convertJsonCard(jsonCard);
    database[card.name] = card;

    // Add alias for Kavaero -> Superior Spider-Man
    if (card.name === "Kavaero, Mind-Bitten") {
      database["Superior Spider-Man"] = { ...card, name: "Superior Spider-Man" };
    }
  }

  return database;
}

const CARD_DATABASE: Record<string, Card> = loadCardDatabase();

// ============================================================================
// DECK REPRESENTATION
// ============================================================================

interface DeckEntry {
  count: number;
  cardName: string;
}

// Parse a deck list file in the format "N Card Name" per line
function parseDeckFile(filename: string): DeckEntry[] {
  const content = readFileSync(filename, "utf-8");
  const deckList: DeckEntry[] = [];

  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) {
      continue; // Skip empty lines and comments
    }

    const match = trimmed.match(/^(\d+)\s+(.+)$/);
    if (match) {
      const count = parseInt(match[1], 10);
      const cardName = match[2].trim();
      deckList.push({ count, cardName });
    }
  }

  return deckList;
}

// Build the full deck array from a deck list
function buildDeckFromList(deckList: DeckEntry[]): Card[] {
  const deck: Card[] = [];
  for (const entry of deckList) {
    const card = CARD_DATABASE[entry.cardName];
    if (!card) {
      throw new Error(`Card not found in database: ${entry.cardName}`);
    }
    for (let i = 0; i < entry.count; i++) {
      deck.push({ ...card }); // Clone to avoid reference issues
    }
  }
  return deck;
}

// Build deck from a file (uses global DECK_FILE if no filename provided)
function buildDeck(filename?: string): Card[] {
  const file = filename ?? DECK_FILE;
  const deckList = parseDeckFile(file);
  return buildDeckFromList(deckList);
}

// ============================================================================
// GAME STATE INITIALIZATION
// ============================================================================

function createInitialGameState(): GameState {
  return {
    library: [],
    hand: [],
    graveyard: [],
    battlefield: [],
    exile: [],
    turn: 0,
    phase: "untap",
    onThePlay: random() < 0.5, // 50% chance
    landPlayedThisTurn: false,
    life: 20,
    opponentLife: 20,
    manaPool: { W: 0, U: 0, B: 0, R: 0, G: 0, C: 0 },
    sagaCounters: new Map(),
  };
}

function emptyManaPool(): ManaPool {
  return { W: 0, U: 0, B: 0, R: 0, G: 0, C: 0 };
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

function shuffle<T>(array: T[]): T[] {
  const result = [...array];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

function countLands(cards: Card[]): number {
  return cards.filter((c) => c.type === "land").length;
}

function drawCards(state: GameState, count: number): void {
  for (let i = 0; i < count && state.library.length > 0; i++) {
    state.hand.push(state.library.shift()!);
  }
}

function mill(state: GameState, count: number): Card[] {
  const milled: Card[] = [];
  for (let i = 0; i < count && state.library.length > 0; i++) {
    const card = state.library.shift()!;
    state.graveyard.push(card);
    milled.push(card);
  }
  return milled;
}

// ============================================================================
// BO1 HAND SMOOTHER
// ============================================================================

function selectOpeningHand(state: GameState): void {
  // Draw two hands of 7
  const hand1 = state.library.splice(0, 7);
  const hand2 = state.library.splice(0, 7);

  const lands1 = countLands(hand1);
  const lands2 = countLands(hand2);

  let chosenHand: Card[];
  let rejectedHand: Card[];

  // Choose hand with fewer lands but at least 2
  if (lands1 >= 2 && lands2 >= 2) {
    // Both valid, pick fewer lands
    if (lands1 < lands2) {
      chosenHand = hand1;
      rejectedHand = hand2;
    } else if (lands2 < lands1) {
      chosenHand = hand2;
      rejectedHand = hand1;
    } else {
      // Same land count, random pick
      if (random() < 0.5) {
        chosenHand = hand1;
        rejectedHand = hand2;
      } else {
        chosenHand = hand2;
        rejectedHand = hand1;
      }
    }
  } else if (lands1 >= 2) {
    chosenHand = hand1;
    rejectedHand = hand2;
  } else if (lands2 >= 2) {
    chosenHand = hand2;
    rejectedHand = hand1;
  } else {
    // Both hands have 0-1 lands, need to mulligan
    // Put both hands back and shuffle
    state.library = shuffle([...state.library, ...hand1, ...hand2]);
    mulliganHand(state, 6);
    return;
  }

  // Put rejected hand back into library and shuffle
  state.library = shuffle([...state.library, ...rejectedHand]);
  state.hand = chosenHand;
}

function mulliganHand(state: GameState, handSize: number): void {
  // Put current hand back if any
  if (state.hand.length > 0) {
    state.library = shuffle([...state.library, ...state.hand]);
    state.hand = [];
  }

  // Draw new hand
  drawCards(state, handSize);

  const lands = countLands(state.hand);
  if (lands < 2 && handSize > 4) {
    // Still bad, mulligan again
    mulliganHand(state, handSize - 1);
    return;
  }

  // Scry for each card below 7
  const scryCount = 7 - handSize;
  if (scryCount > 0) {
    // For now, simple scry: bottom any Bringer/Terror, keep lands/enablers
    scryAfterMulligan(state, scryCount);
  }
}

function scryAfterMulligan(state: GameState, count: number): void {
  // Look at top N cards
  const topCards = state.library.slice(0, count);
  const toBottom: Card[] = [];
  const toTop: Card[] = [];

  for (const card of topCards) {
    // Bottom: Bringer/Terror (want in graveyard, not hand)
    if (
      card.name === "Bringer of the Last Gift" ||
      card.name === "Terror of the Peaks"
    ) {
      toBottom.push(card);
    } else {
      toTop.push(card);
    }
  }

  // Reconstruct library: toTop first, then rest, then toBottom
  const restOfLibrary = state.library.slice(count);
  state.library = [...toTop, ...restOfLibrary, ...toBottom];
}

// ============================================================================
// PHASE 2: MANA SYSTEM
// ============================================================================

// Helper to check if a creature matches Cavern's chosen type
function creatureMatchesCavernType(creature: CreatureCard, chosenType: string): boolean {
  return creature.creatureTypes.includes(chosenType);
}

// Helper to determine if a land will enter untapped given current game state
function willLandEnterUntapped(land: LandCard, state: GameState): boolean {
  if (land.entersTapped === false) return true;
  if (land.entersTapped === true) return false;

  // Conditional - depends on land subtype
  if (land.subtype === "fastland") {
    // Blooming Marsh: enters untapped if ≤2 other lands
    const otherLands = state.battlefield.filter(p => p.card.type === "land").length;
    return otherLands <= 2;
  }
  if (land.subtype === "town") {
    // Starting Town: enters untapped on turns 1-3
    return state.turn <= 3;
  }
  // Shock lands, Multiversal Passage: pay 2 life to enter untapped
  return state.life > 2;
}

function canTapForMana(
  permanent: BattlefieldPermanent,
  state: GameState,
  forCreature?: CreatureCard // Optional: the creature we're trying to cast
): ManaColor[] {
  if (permanent.tapped) return [];
  if (permanent.card.type !== "land") return [];

  const land = permanent.card as LandCard;

  // Handle Cavern of Souls - colored mana ONLY for creatures of chosen type
  if (land.name === "Cavern of Souls") {
    // Cavern always produces {C}
    // Cavern produces any color ONLY for creatures of the chosen type
    if (forCreature && permanent.chosenType && creatureMatchesCavernType(forCreature, permanent.chosenType)) {
      // Creature matches! Can produce any color
      return ["W", "U", "B", "R", "G", "C"];
    }
    // No creature context or creature doesn't match - only colorless
    return ["C"];
  }

  // Handle Wastewood Verge - {B} only if controlling Swamp/Forest
  if (land.name === "Wastewood Verge") {
    const hasSwampOrForest = state.battlefield.some((p) => {
      if (p.card.type !== "land") return false;
      const l = p.card as LandCard;
      return (
        l.name === "Swamp" ||
        l.name === "Forest" ||
        l.name === "Watery Grave" ||
        l.name === "Underground Mortuary" ||
        l.name === "Undercity Sewers" // Has Swamp type
      );
    });
    if (hasSwampOrForest) {
      return ["G", "B"];
    }
    return ["G"];
  }

  // Handle Gloomlake Verge - {B} only if controlling Island/Swamp
  if (land.name === "Gloomlake Verge") {
    const hasIslandOrSwamp = state.battlefield.some((p) => {
      if (p.card.type !== "land") return false;
      const l = p.card as LandCard;
      return (
        l.name === "Island" ||
        l.name === "Swamp" ||
        l.name === "Watery Grave" ||
        l.name === "Undercity Sewers" // Has Island/Swamp type
      );
    });
    if (hasIslandOrSwamp) {
      return ["U", "B"];
    }
    return ["U"];
  }

  // Handle Multiversal Passage - produces chosen color
  if (land.name === "Multiversal Passage" && permanent.chosenBasicType) {
    return [permanent.chosenBasicType];
  }

  // Handle Starting Town - produces C for free, or any color for 1 life
  if (land.name === "Starting Town") {
    if (state.life > 1) {
      // Can pay 1 life for any color
      return ["C", "W", "U", "B", "R", "G"];
    }
    // Only colorless if we can't afford the life
    return ["C"];
  }

  return [...land.colors];
}

function tapLandForMana(
  permanent: BattlefieldPermanent,
  color: ManaColor,
  state: GameState,
  forCreature?: CreatureCard
): boolean {
  const available = canTapForMana(permanent, state, forCreature);
  if (!available.includes(color)) return false;

  const land = permanent.card as LandCard;

  // Starting Town: pay 1 life for colored mana
  if (land.name === "Starting Town" && color !== "C") {
    if (state.life <= 1) return false;
    state.life -= 1;
  }

  permanent.tapped = true;
  state.manaPool[color]++;
  return true;
}

function getTotalMana(pool: ManaPool): number {
  return pool.W + pool.U + pool.B + pool.R + pool.G + pool.C;
}

function canPayManaCost(cost: ManaCost, pool: ManaPool): boolean {
  // First check colored requirements
  if ((cost.W || 0) > pool.W) return false;
  if ((cost.U || 0) > pool.U) return false;
  if ((cost.B || 0) > pool.B) return false;
  if ((cost.R || 0) > pool.R) return false;
  if ((cost.G || 0) > pool.G) return false;
  if ((cost.C || 0) > pool.C) return false;

  // Then check if we have enough left for generic
  const remainingAfterColors =
    pool.W -
    (cost.W || 0) +
    (pool.U - (cost.U || 0)) +
    (pool.B - (cost.B || 0)) +
    (pool.R - (cost.R || 0)) +
    (pool.G - (cost.G || 0)) +
    (pool.C - (cost.C || 0));

  return remainingAfterColors >= (cost.generic || 0);
}

function payManaCost(cost: ManaCost, state: GameState): boolean {
  if (!canPayManaCost(cost, state.manaPool)) return false;

  // Pay colored costs first
  state.manaPool.W -= cost.W || 0;
  state.manaPool.U -= cost.U || 0;
  state.manaPool.B -= cost.B || 0;
  state.manaPool.R -= cost.R || 0;
  state.manaPool.G -= cost.G || 0;
  state.manaPool.C -= cost.C || 0;

  // Pay generic with remaining mana (prefer colorless, then excess colors)
  let genericRemaining = cost.generic || 0;
  const colors: ManaColor[] = ["C", "W", "U", "B", "R", "G"];
  for (const color of colors) {
    const available = state.manaPool[color];
    const toPay = Math.min(available, genericRemaining);
    state.manaPool[color] -= toPay;
    genericRemaining -= toPay;
    if (genericRemaining <= 0) break;
  }

  return true;
}

// Calculate maximum available mana from untapped lands
function getAvailableMana(state: GameState): ManaPool {
  const available = emptyManaPool();
  for (const permanent of state.battlefield) {
    if (permanent.tapped) continue;
    if (permanent.card.type !== "land") continue;
    const colors = canTapForMana(permanent, state);
    // For simplicity, just count each land once with its first color
    // Real optimization would consider all possibilities
    for (const color of colors) {
      available[color]++;
      break; // Count land once
    }
  }
  return available;
}

function getMaxAvailableMana(state: GameState): number {
  return state.battlefield.filter(
    (p) => p.card.type === "land" && !p.tapped
  ).length;
}

// ============================================================================
// PHASE 2: LAND PLAYING
// ============================================================================

function playLand(state: GameState, card: Card): boolean {
  if (state.landPlayedThisTurn) return false;
  if (card.type !== "land") return false;

  const handIndex = state.hand.findIndex((c) => c === card);
  if (handIndex === -1) return false;

  // Remove from hand
  state.hand.splice(handIndex, 1);

  const land = card as LandCard;
  let entersTapped = false;

  // Handle conditional ETB based on land subtype
  if (land.entersTapped === true) {
    entersTapped = true;
  } else if (land.entersTapped === "conditional") {
    // Different logic based on land subtype
    if (land.subtype === "fastland") {
      // Blooming Marsh: enters untapped if ≤2 other lands
      const otherLands = state.battlefield.filter(p => p.card.type === "land").length;
      entersTapped = otherLands > 2;
    } else if (land.subtype === "town") {
      // Starting Town: enters untapped on turns 1-3
      entersTapped = state.turn > 3;
    } else {
      // Shock lands, Multiversal Passage: pay 2 life to enter untapped
      if (state.life > 2) {
        state.life -= 2;
        entersTapped = false;
      } else {
        entersTapped = true;
      }
    }
  }

  const permanent: BattlefieldPermanent = {
    card,
    tapped: entersTapped,
    turnEntered: state.turn,
  };

  // Handle choices for lands
  if (land.name === "Cavern of Souls") {
    // Choose creature type based on what creatures are in hand that we might want to cast
    // Priority: Human (Spider-Man, Town Greeter) > Demon (Bringer) > Noble (Kiora) > Avatar (Overlord)
    const creaturesInHand = state.hand.filter(c => c.type === "creature") as CreatureCard[];

    // Check if we already have a Cavern with Human type
    const existingCaverns = state.battlefield.filter(p =>
      p.card.name === "Cavern of Souls" && p.chosenType
    );
    const hasHumanCavern = existingCaverns.some(p => p.chosenType === "Human");

    // Check if we have another Cavern in hand (so we can save Human for later)
    const cavernsInHand = state.hand.filter(c => c.name === "Cavern of Souls");
    const hasKioraInHand = creaturesInHand.some(c => c.name === "Kiora, the Rising Tide");
    const hasBringerOrTerrorInHand = creaturesInHand.some(c =>
      c.name === "Bringer of the Last Gift" || c.name === "Terror of the Peaks"
    );

    let chosenType = "Human"; // Default

    // Special case: If we have Kiora + Bringer/Terror in hand (want to cast Kiora first to discard)
    // AND we have another Cavern coming, set this one to Noble
    if (!hasHumanCavern && hasKioraInHand && hasBringerOrTerrorInHand && cavernsInHand.length >= 1) {
      chosenType = "Noble"; // Cast Kiora first to discard Bringer/Terror
    } else if (hasHumanCavern) {
      // We already have Human covered, pick something else based on hand
      if (creaturesInHand.some(c => c.name === "Bringer of the Last Gift")) {
        chosenType = "Demon";
      } else if (creaturesInHand.some(c => c.name === "Kiora, the Rising Tide")) {
        chosenType = "Noble";
      } else if (creaturesInHand.some(c => c.name === "Overlord of the Balemurk")) {
        chosenType = "Avatar";
      } else if (creaturesInHand.some(c => c.name === "Terror of the Peaks")) {
        chosenType = "Dragon";
      } else {
        // No specific need, default to Demon (in case we draw Bringer)
        chosenType = "Demon";
      }
    } else {
      // First Cavern - default to Human (helps Spider-Man and Town Greeter)
      chosenType = "Human";
    }

    permanent.chosenType = chosenType;
    if (VERBOSE) {
      console.log(`    (Cavern set to: ${chosenType})`);
    }
  } else if (land.name === "Multiversal Passage") {
    // Choose based on what spells we can cast this turn
    // Check what colors we're missing to cast spells in hand
    const untappedLands = state.battlefield.filter(p =>
      p.card.type === "land" && !p.tapped
    );
    const availableColors = new Set<ManaColor>();
    for (const p of untappedLands) {
      for (const c of canTapForMana(p, state)) {
        availableColors.add(c);
      }
    }

    // Check if we need G for Cache Grab, Town Greeter, or Analyze the Pollen
    const needsGreen = state.hand.some(c => c.manaCost?.G && c.manaCost.G > 0);
    const hasGreen = availableColors.has("G");

    // Check if we need U for Kiora, Spider-Man, Dredger's Insight
    const needsBlue = state.hand.some(c => c.manaCost?.U && c.manaCost.U > 0);
    const hasBlue = availableColors.has("U");

    // Check if we need B for Spider-Man, Overlord
    const needsBlack = state.hand.some(c => c.manaCost?.B && c.manaCost.B > 0);
    const hasBlack = availableColors.has("B");

    // Priority: Fill missing colors for castable spells
    if (needsGreen && !hasGreen) {
      permanent.chosenBasicType = "G";
    } else if (needsBlue && !hasBlue) {
      permanent.chosenBasicType = "U";
    } else if (needsBlack && !hasBlack) {
      permanent.chosenBasicType = "B";
    } else if (!hasBlue) {
      // Default: prioritize blue for Spider-Man and Kiora
      permanent.chosenBasicType = "U";
    } else if (!hasBlack) {
      permanent.chosenBasicType = "B";
    } else if (!hasGreen) {
      permanent.chosenBasicType = "G";
    } else {
      permanent.chosenBasicType = "U"; // fallback
    }
  }

  state.battlefield.push(permanent);
  state.landPlayedThisTurn = true;

  if (VERBOSE) {
    let msg = `  [Land] ${land.name}`;
    if (entersTapped) msg += " (tapped)";
    if (land.hasSurveil) msg += " (surveil)";
    if (land.name === "Multiversal Passage" && permanent.chosenBasicType) {
      const typeNames: Record<ManaColor, string> = { W: "Plains", U: "Island", B: "Swamp", R: "Mountain", G: "Forest" };
      msg += ` (chose: ${typeNames[permanent.chosenBasicType]})`;
    }
    console.log(msg);
  }

  // Handle surveil ETB triggers
  if (land.hasSurveil && land.surveilAmount) {
    resolveSurveil(state, land.surveilAmount);
  }

  return true;
}

// ============================================================================
// PHASE 2: SURVEIL MECHANIC
// ============================================================================

function resolveSurveil(state: GameState, count: number): void {
  const toGraveyard: Card[] = [];
  const toTop: Card[] = [];

  for (let i = 0; i < count && state.library.length > 0; i++) {
    const topCard = state.library[0];

    // Decision: keep on top or put in graveyard?
    // Graveyard: Bringer, Terror, Overlord (want to reanimate these)
    // Also put Kiora if we already have one (for reanimation value)
    // Top: Spider-Man (MUST stay in hand!), lands, mill spells
    const hasKioraInHand = state.hand.some(c => c.name === "Kiora, the Rising Tide");
    const putInGraveyard =
      topCard.name === "Bringer of the Last Gift" ||
      topCard.name === "Terror of the Peaks" ||
      topCard.name === "Overlord of the Balemurk" ||
      (topCard.name === "Kiora, the Rising Tide" && hasKioraInHand) ||
      topCard.name === "Town Greeter"; // Cheap 1/1, better to reanimate than draw

    if (putInGraveyard) {
      state.library.shift();
      state.graveyard.push(topCard);
      toGraveyard.push(topCard);
    } else {
      toTop.push(topCard);
    }
    // If keeping on top, just leave it there
  }

  if (VERBOSE && (toGraveyard.length > 0 || toTop.length > 0)) {
    if (toGraveyard.length > 0) {
      console.log(`    Surveil -> graveyard: ${toGraveyard.map(c => c.name).join(", ")}`);
    }
    if (toTop.length > 0) {
      console.log(`    Surveil -> kept on top: ${toTop.map(c => c.name).join(", ")}`);
    }
  }
}

// ============================================================================
// PHASE 3: CARD ABILITY IMPLEMENTATIONS
// ============================================================================

// --- Mill with Selection ---
// Mill N cards, optionally return one matching a filter to hand
function millWithSelection(
  state: GameState,
  count: number,
  filter: (card: Card) => boolean,
  selectBest: (cards: Card[], state: GameState) => Card | null
): Card[] {
  const milled = mill(state, count);
  const validChoices = milled.filter(filter);

  if (VERBOSE) {
    console.log(`    Mill ${count}: ${milled.map(c => c.name).join(", ")}`);
  }

  if (validChoices.length > 0) {
    const selected = selectBest(validChoices, state);
    if (selected) {
      // Remove from graveyard, add to hand
      const idx = state.graveyard.indexOf(selected);
      if (idx !== -1) {
        state.graveyard.splice(idx, 1);
        state.hand.push(selected);
        if (VERBOSE) {
          console.log(`    -> Returned to hand: ${selected.name}`);
        }
      }
    }
  }

  return milled;
}

// Selection heuristics for mill effects
function selectBestFromMill(cards: Card[], state: GameState): Card | null {
  if (cards.length === 0) return null;

  const hasBringerInGraveyard = state.graveyard.some(
    (c) => c.name === "Bringer of the Last Gift"
  );
  const hasSpiderManInHand = state.hand.some(
    (c) => c.name === "Superior Spider-Man"
  );
  const hasBringerInHand = state.hand.some(
    (c) => c.name === "Bringer of the Last Gift"
  );
  const landCount = state.battlefield.filter(
    (p) => p.card.type === "land"
  ).length;
  const landsInHand = state.hand.filter((c) => c.type === "land").length;

  // Priority:
  // 1. Superior Spider-Man - ALWAYS grab it (key combo piece), unless we already have one
  // 2. Kiora if Bringer is in hand (need to discard it)
  // 3. Lands ONLY if we're desperate (0-1 lands on battlefield and none in hand)
  // 4. Other enablers

  for (const card of cards) {
    // ALWAYS get Spider-Man - it's the key combo enabler
    if (card.name === "Superior Spider-Man" && !hasSpiderManInHand) {
      return card;
    }
  }

  for (const card of cards) {
    // Get Kiora if Bringer is stuck in hand
    if (card.name === "Kiora, the Rising Tide" && hasBringerInHand) {
      return card;
    }
  }

  // Only get land if we're desperate (very few lands and none in hand)
  const desperateForLand = landCount <= 1 && landsInHand === 0;
  if (desperateForLand) {
    const land = cards.find((c) => c.type === "land");
    if (land) return land;
  }

  // Otherwise, get mill enablers (creatures that help us mill more)
  const enabler = cards.find(
    (c) =>
      c.type === "creature" &&
      (c.name === "Town Greeter" ||
        c.name === "Overlord of the Balemurk" ||
        c.name === "Kiora, the Rising Tide")
  );
  if (enabler) return enabler;

  // Get land if we need it (< 4 lands)
  if (landCount < 4) {
    const land = cards.find((c) => c.type === "land");
    if (land) return land;
  }

  // Get any non-combo creature (but NEVER return Bringer or Terror - we want them in the graveyard!)
  const creature = cards.find(
    (c) =>
      c.type === "creature" &&
      c.name !== "Bringer of the Last Gift" &&
      c.name !== "Terror of the Peaks"
  );
  if (creature) return creature;

  // Get any permanent EXCEPT combo pieces (Bringer, Terror)
  // These should stay in the graveyard for reanimation
  const permanent = cards.find(
    (c) =>
      c.type !== "instant" &&
      c.type !== "sorcery" &&
      c.name !== "Bringer of the Last Gift" &&
      c.name !== "Terror of the Peaks"
  );
  return permanent || null;
}

function selectBestLand(cards: Card[], state: GameState): Card | null {
  const lands = cards.filter((c) => c.type === "land");
  if (lands.length === 0) return null;

  // Prefer untapped lands, then dual lands
  lands.sort((a, b) => {
    const aLand = a as LandCard;
    const bLand = b as LandCard;
    if (aLand.entersTapped !== bLand.entersTapped) {
      return aLand.entersTapped ? 1 : -1;
    }
    // Prefer multi-color
    return bLand.colors.length - aLand.colors.length;
  });

  return lands[0];
}

// --- Creature ETB Triggers ---

function resolveCreatureETB(
  state: GameState,
  permanent: BattlefieldPermanent
): void {
  const card = permanent.card as CreatureCard;

  switch (card.name) {
    case "Kiora, the Rising Tide":
      resolveKioraETB(state);
      break;
    case "Town Greeter":
      resolveTownGreeterETB(state);
      break;
    case "Overlord of the Balemurk":
      // Triggers on enter even with impending (says "this permanent enters", not "this creature")
      resolveOverlordETB(state);
      break;
    case "Superior Spider-Man":
      resolveSpiderManETB(state, permanent);
      break;
    case "Bringer of the Last Gift":
      // Only triggers if cast (not reanimated) - we'll handle this specially
      // For now, assume it was cast if it's entering from a cast
      resolveBringerETB(state, permanent);
      break;
    case "Terror of the Peaks":
      // Terror doesn't have an ETB, but triggers when OTHER creatures enter
      break;
  }
}

function resolveKioraETB(state: GameState): void {
  // Draw 2, discard 2
  const handBefore = state.hand.length;
  drawCards(state, 2);
  const drawn = state.hand.slice(handBefore);

  if (VERBOSE) {
    console.log(`    Kiora ETB: drew ${drawn.map(c => c.name).join(", ")}`);
  }

  // Discard 2 - prioritize discarding Bringer/Terror
  const discarded: Card[] = [];
  for (let i = 0; i < 2 && state.hand.length > 0; i++) {
    // Find best card to discard
    let toDiscard = state.hand.find(
      (c) => c.name === "Bringer of the Last Gift"
    );
    if (!toDiscard) {
      toDiscard = state.hand.find((c) => c.name === "Terror of the Peaks");
    }
    if (!toDiscard) {
      // Discard excess lands or least useful card
      const lands = state.hand.filter((c) => c.type === "land");
      if (lands.length > 2) {
        toDiscard = lands[lands.length - 1];
      } else {
        // Discard last card
        toDiscard = state.hand[state.hand.length - 1];
      }
    }

    const idx = state.hand.indexOf(toDiscard);
    if (idx !== -1) {
      state.hand.splice(idx, 1);
      state.graveyard.push(toDiscard);
      discarded.push(toDiscard);
    }
  }

  if (VERBOSE) {
    console.log(`    Kiora ETB: discarded ${discarded.map(c => c.name).join(", ")}`);
  }
}

function resolveTownGreeterETB(state: GameState): void {
  // Mill 4, may put a land into hand
  millWithSelection(
    state,
    4,
    (c) => c.type === "land",
    selectBestLand
  );
}

function resolveOverlordETB(state: GameState): void {
  // Mill 4, return non-Avatar creature or planeswalker
  // BUT we usually DON'T want to return creatures - we want them in graveyard for reanimate!
  millWithSelection(
    state,
    4,
    (c) => {
      if (c.type !== "creature") return false;
      const creature = c as CreatureCard;
      // Non-Avatar creature
      return !creature.creatureTypes.includes("Avatar");
    },
    selectForOverlord // Use specialized selector that's more restrictive
  );
}

// Specialized selection for Overlord - most of the time we DON'T want to return creatures
function selectForOverlord(cards: Card[], state: GameState): Card | null {
  if (cards.length === 0) return null;

  const hasBringerInGraveyard = state.graveyard.some(
    (c) => c.name === "Bringer of the Last Gift"
  );
  const hasSpiderManInHand = state.hand.some(
    (c) => c.name === "Superior Spider-Man"
  );
  const hasBringerInHand = state.hand.some(
    (c) => c.name === "Bringer of the Last Gift"
  );

  // Priority 1: Superior Spider-Man if we need it for the combo
  if (hasBringerInGraveyard && !hasSpiderManInHand) {
    const spiderMan = cards.find((c) => c.name === "Superior Spider-Man");
    if (spiderMan) {
      if (VERBOSE) console.log(`    Overlord returns Superior Spider-Man (combo piece!)`);
      return spiderMan;
    }
  }

  // Priority 2: Kiora if Bringer is stuck in hand (need to discard it)
  if (hasBringerInHand) {
    const kiora = cards.find((c) => c.name === "Kiora, the Rising Tide");
    if (kiora) {
      if (VERBOSE) console.log(`    Overlord returns Kiora (need to discard Bringer from hand)`);
      return kiora;
    }
  }

  // Priority 3: Town Greeter - cheap enabler that can mill more
  // Only get it if we're early game and need to keep milling
  const landCount = state.battlefield.filter((p) => p.card.type === "land").length;
  if (landCount < 4) {
    const townGreeter = cards.find((c) => c.name === "Town Greeter");
    if (townGreeter) {
      if (VERBOSE) console.log(`    Overlord returns Town Greeter (cheap enabler)`);
      return townGreeter;
    }
  }

  // Otherwise: DON'T return anything! Leave creatures in graveyard for reanimation
  if (VERBOSE) console.log(`    Overlord returns nothing (keeping creatures for reanimate)`);
  return null;
}

function resolveSpiderManETB(
  state: GameState,
  permanent: BattlefieldPermanent
): void {
  // May enter as a copy of any creature in a graveyard
  // Priority: Copy Bringer if in graveyard (THE COMBO!)

  const bringerInGraveyard = state.graveyard.find(
    (c) => c.name === "Bringer of the Last Gift"
  );

  if (bringerInGraveyard) {
    if (VERBOSE) {
      console.log(`    *** COMBO! Superior Spider-Man copies Bringer of the Last Gift! ***`);
    }

    // Copy Bringer!
    permanent.isCopyOf = "Bringer of the Last Gift";

    // Exile the copied card
    const idx = state.graveyard.indexOf(bringerInGraveyard);
    if (idx !== -1) {
      state.graveyard.splice(idx, 1);
      state.exile.push(bringerInGraveyard);
    }

    // Now trigger Bringer's ETB (mass reanimate!)
    resolveBringerETB(state, permanent);
  } else if (VERBOSE) {
    console.log(`    Spider-Man enters as a 4/4 (no good copy target)`);
  }
  // If no good target, Spider-Man just enters as a 4/4
}

function resolveBringerETB(
  state: GameState,
  permanent: BattlefieldPermanent
): void {
  // Sacrifice all other creatures, then return all creatures from graveyard

  // Step 1: Sacrifice all other creatures (move to graveyard)
  // NOTE: Impending creatures (with time counters) are NOT creatures yet - they're enchantments!
  const toSacrifice = state.battlefield.filter(
    (p) => p !== permanent && p.card.type === "creature" && (p.counters ?? 0) === 0
  );

  // Check for impending creatures that survive
  const impendingSurvivors = state.battlefield.filter(
    (p) => p !== permanent && p.card.type === "creature" && (p.counters ?? 0) > 0
  );

  if (VERBOSE && toSacrifice.length > 0) {
    console.log(`    Sacrifice: ${toSacrifice.map(p => p.card.name).join(", ")}`);
  }
  if (VERBOSE && impendingSurvivors.length > 0) {
    console.log(`    Impending survives: ${impendingSurvivors.map(p => `${p.card.name} (${p.counters} counters)`).join(", ")}`);
  }

  for (const p of toSacrifice) {
    const idx = state.battlefield.indexOf(p);
    if (idx !== -1) {
      state.battlefield.splice(idx, 1);
      state.graveyard.push(p.card);
    }
  }

  // Step 2: Return ALL creature cards from graveyard to battlefield
  // (except the ones just sacrificed - but we'll simplify and return all)
  const creaturesToReanimate = state.graveyard.filter(
    (c) => c.type === "creature"
  );

  if (VERBOSE && creaturesToReanimate.length > 0) {
    console.log(`    Reanimate: ${creaturesToReanimate.map(c => c.name).join(", ")}`);
  }

  // Remove from graveyard
  state.graveyard = state.graveyard.filter((c) => c.type !== "creature");

  // Add to battlefield - each one triggers Terror of the Peaks!
  const terrorOnField = state.battlefield.find(
    (p) =>
      p.card.name === "Terror of the Peaks" ||
      (p.isCopyOf === "Terror of the Peaks")
  );

  // Also check if we're reanimating Terror
  const terrorInReanimated = creaturesToReanimate.find(
    (c) => c.name === "Terror of the Peaks"
  );

  // Reanimate all creatures
  for (const creature of creaturesToReanimate) {
    const newPermanent: BattlefieldPermanent = {
      card: creature,
      tapped: false,
      turnEntered: state.turn,
    };
    state.battlefield.push(newPermanent);
  }

  // Now resolve Terror triggers for each creature that entered
  // Terror deals damage equal to each creature's power
  resolveTerrorTriggers(state, creaturesToReanimate);
}

function resolveTerrorTriggers(state: GameState, entering: Card[]): void {
  // Count how many Terrors are on the battlefield
  const terrorCount = state.battlefield.filter(
    (p) =>
      p.card.name === "Terror of the Peaks" ||
      p.isCopyOf === "Terror of the Peaks"
  ).length;

  if (terrorCount === 0) return;

  // Each Terror triggers for each OTHER creature entering
  // (Terror doesn't trigger for itself)
  let totalDamage = 0;

  for (const creature of entering) {
    if (creature.name === "Terror of the Peaks") continue; // Doesn't trigger for itself

    const creatureCard = creature as CreatureCard;
    // Each Terror deals damage equal to the creature's power
    totalDamage += creatureCard.power * terrorCount;
  }

  state.opponentLife -= totalDamage;

  if (VERBOSE && totalDamage > 0) {
    console.log(
      `  Terror triggers dealt ${totalDamage} damage! (${terrorCount} Terror(s), ${entering.length} creatures entered)`
    );
  }
}

// Calculate expected damage from casting Spider-Man copying Bringer THIS TURN
// Only counts immediate damage: Terror triggers + combat from non-summoning-sick creatures
function calculateComboDamage(state: GameState): number {
  // Creatures that would be reanimated from graveyard
  const creaturesInGraveyard = state.graveyard.filter(
    (c): c is CreatureCard => c.type === "creature"
  );

  // Spider-Man copies Bringer (power 6), and Bringer (the copied one) is exiled
  const bringerPower = 6;

  // Count Terrors that will be on battlefield after combo
  const terrorsInGraveyard = creaturesInGraveyard.filter(
    (c) => c.name === "Terror of the Peaks"
  ).length;
  const terrorsOnBattlefield = state.battlefield.filter(
    (p) =>
      p.card.name === "Terror of the Peaks" ||
      p.isCopyOf === "Terror of the Peaks"
  ).length;
  const totalTerrors = terrorsInGraveyard + terrorsOnBattlefield;

  // Calculate Terror trigger damage (IMMEDIATE)
  // When Spider-Man enters as a copy of Bringer, creatures are reanimated
  // Each Terror triggers for each creature entering (except itself)
  //
  // IMPORTANT: Spider-Man entering does NOT trigger Terrors because Terror is
  // still in the graveyard at that point. Terrors only trigger for the creatures
  // that enter simultaneously with them during the mass reanimate.

  let terrorDamage = 0;

  // Terrors already on battlefield trigger for EACH creature entering (including Spider-Man)
  if (terrorsOnBattlefield > 0) {
    terrorDamage += bringerPower * terrorsOnBattlefield;
    for (const creature of creaturesInGraveyard) {
      terrorDamage += creature.power * terrorsOnBattlefield;
    }
  }

  // Terrors from graveyard trigger for creatures entering AT THE SAME TIME (during mass reanimate)
  // They DON'T trigger for Spider-Man (Spider-Man entered BEFORE the mass reanimate)
  // They trigger for all other creatures entering simultaneously, but NOT for themselves
  if (terrorsInGraveyard > 0) {
    // Each creature from graveyard triggers Terrors from graveyard (except Terror doesn't trigger for itself)
    for (const creature of creaturesInGraveyard) {
      if (creature.name === "Terror of the Peaks") {
        // A Terror entering triggers all OTHER Terrors (from graveyard only - battlefield Terrors already triggered above)
        terrorDamage += creature.power * (terrorsInGraveyard - 1);
      } else {
        terrorDamage += creature.power * terrorsInGraveyard;
      }
    }
  }

  // Combat damage from creatures that can attack THIS turn (already on battlefield, no summoning sickness)
  // These creatures will attack after we cast the combo in main phase 1
  const currentCombatPower = state.battlefield
    .filter((p) => {
      if (p.card.type !== "creature") return false;
      if (p.counters && p.counters > 0) return false; // Impending
      return state.turn > p.turnEntered; // No summoning sickness
    })
    .reduce(
      (sum, p) => sum + ((p.card as CreatureCard).power || 0),
      0
    );

  // Reanimated creatures have summoning sickness and CAN'T attack this turn
  // So we don't count them for this turn's damage

  return terrorDamage + currentCombatPower;
}

// Check if casting the combo NOW would be lethal
function isComboLethal(state: GameState): boolean {
  const expectedDamage = calculateComboDamage(state);
  if (VERBOSE && expectedDamage > 0 && expectedDamage < state.opponentLife) {
    // Debug: show the breakdown when close to lethal
    const creaturesInGraveyard = state.graveyard.filter(
      (c): c is CreatureCard => c.type === "creature"
    );
    const terrorsInGraveyard = creaturesInGraveyard.filter(
      (c) => c.name === "Terror of the Peaks"
    ).length;
    const currentCombatPower = state.battlefield
      .filter((p) => {
        if (p.card.type !== "creature") return false;
        if (p.counters && p.counters > 0) return false;
        return state.turn > p.turnEntered;
      })
      .reduce((sum, p) => sum + ((p.card as CreatureCard).power || 0), 0);
    console.log(
      `    [Damage calc] Terrors in GY: ${terrorsInGraveyard}, Combat power: ${currentCombatPower}, Total: ${expectedDamage}`
    );
  }
  return expectedDamage >= state.opponentLife;
}

// --- Spell Resolution ---

function resolveSpellAbility(state: GameState, card: Card): void {
  switch (card.name) {
    case "Cache Grab":
      // Mill 4, return a permanent to hand
      millWithSelection(
        state,
        4,
        (c) => c.type !== "instant" && c.type !== "sorcery",
        selectBestFromMill
      );
      break;

    case "Dredger's Insight":
      // ETB: Mill 4, return artifact/creature/land
      millWithSelection(
        state,
        4,
        (c) =>
          c.type === "creature" || c.type === "land" || c.type === "enchantment",
        selectBestFromMill
      );
      break;

    case "Analyze the Pollen":
      resolveAnalyzeThePollen(state);
      break;
  }
}

function resolveAnalyzeThePollen(state: GameState): void {
  // May collect evidence 8 (exile 8+ MV from graveyard)
  // If evidence collected, search for creature or land; otherwise just basic land

  // NEVER exile: Terror, Bringer (combo pieces), lands (MV 0, don't help)
  const neverExile = ["Terror of the Peaks", "Bringer of the Last Gift"];
  const exilableCards = state.graveyard.filter(
    c => c.type !== "land" && !neverExile.includes(c.name)
  );

  // Calculate MV of exilable cards only
  const exilableMV = exilableCards.reduce((sum, c) => sum + c.manaValue, 0);
  const canCollectEvidence = exilableMV >= 8;

  if (canCollectEvidence) {
    // Collect evidence - exile cards totaling 8+ MV
    let evidenceMV = 0;
    const toExile: Card[] = [];

    // Sort by what we want to exile
    // Priority: Spells > Enchantments > Creatures (minimize creature exile)
    const sortedExilable = [...exilableCards].sort((a, b) => {
      // Prefer exiling spells/sorceries over creatures
      const typeOrder = (c: Card) => {
        if (c.type === "sorcery" || c.type === "instant") return 0;
        if (c.type === "enchantment" || c.type === "saga") return 1;
        if (c.type === "creature") return 2;
        return 3;
      };
      const orderDiff = typeOrder(a) - typeOrder(b);
      if (orderDiff !== 0) return orderDiff;
      // Within same type, prefer higher MV to reach 8 faster
      return b.manaValue - a.manaValue;
    });

    for (const card of sortedExilable) {
      if (evidenceMV >= 8) break;
      toExile.push(card);
      evidenceMV += card.manaValue;
    }

    // Exile the cards
    for (const card of toExile) {
      const idx = state.graveyard.indexOf(card);
      if (idx !== -1) {
        state.graveyard.splice(idx, 1);
        state.exile.push(card);
      }
    }

    if (VERBOSE) {
      console.log(`    Evidence collected (${evidenceMV} MV exiled: ${toExile.map(c => c.name).join(", ")})`);
    }

    // Search for creature or land
    // Priority: Superior Spider-Man if we need it, else Kiora, else land
    const hasSpiderMan = state.hand.some(
      (c) => c.name === "Superior Spider-Man"
    );
    const hasBringerInGY = state.graveyard.some(
      (c) => c.name === "Bringer of the Last Gift"
    );

    let target: Card | undefined;

    if (!hasSpiderMan && hasBringerInGY) {
      // Search for Spider-Man
      target = state.library.find((c) => c.name === "Superior Spider-Man");
    }

    if (!target) {
      // Search for Kiora
      target = state.library.find((c) => c.name === "Kiora, the Rising Tide");
    }

    if (!target) {
      // Search for a land
      target = state.library.find((c) => c.type === "land");
    }

    if (target) {
      const idx = state.library.indexOf(target);
      state.library.splice(idx, 1);
      state.hand.push(target);
      // Shuffle library
      state.library = shuffle(state.library);
      if (VERBOSE) {
        console.log(`    -> Searched for: ${target.name}`);
      }
    }
  } else {
    // No evidence - just search for basic land
    if (VERBOSE) {
      console.log(`    No evidence (graveyard MV: ${graveyardMV}/8)`);
    }
    const basicLand = state.library.find(
      (c) =>
        c.type === "land" && (c as LandCard).subtype === "basic"
    );
    if (basicLand) {
      const idx = state.library.indexOf(basicLand);
      state.library.splice(idx, 1);
      state.hand.push(basicLand);
      state.library = shuffle(state.library);
      if (VERBOSE) {
        console.log(`    -> Searched for basic land: ${basicLand.name}`);
      }
    } else {
      if (VERBOSE) {
        console.log(`    -> No basic land found in library`);
      }
    }
  }
}

// --- Saga Resolution ---

function resolveSagaChapter(
  state: GameState,
  permanent: BattlefieldPermanent,
  chapter: number
): void {
  const saga = permanent.card as SagaCard;

  if (saga.name === "Awaken the Honored Dead") {
    switch (chapter) {
      case 1:
        // Destroy target nonland permanent - we're goldfishing so nothing to destroy
        break;
      case 2:
        // Mill 3
        mill(state, 3);
        break;
      case 3:
        // May discard a card, if you do return creature or land from GY to hand
        // Discard Bringer/Terror if in hand
        const toDiscard = state.hand.find(
          (c) =>
            c.name === "Bringer of the Last Gift" ||
            c.name === "Terror of the Peaks"
        );
        if (toDiscard) {
          const idx = state.hand.indexOf(toDiscard);
          state.hand.splice(idx, 1);
          state.graveyard.push(toDiscard);

          // Return creature or land from GY
          const toReturn = state.graveyard.find(
            (c) =>
              (c.type === "creature" &&
                c.name !== "Bringer of the Last Gift" &&
                c.name !== "Terror of the Peaks") ||
              c.type === "land"
          );
          if (toReturn) {
            const retIdx = state.graveyard.indexOf(toReturn);
            state.graveyard.splice(retIdx, 1);
            state.hand.push(toReturn);
          }
        }
        // Sacrifice saga after chapter 3
        const sagaIdx = state.battlefield.indexOf(permanent);
        if (sagaIdx !== -1) {
          state.battlefield.splice(sagaIdx, 1);
          state.graveyard.push(permanent.card);
        }
        break;
    }
  }
}

// ============================================================================
// PHASE 2: SPELL CASTING
// ============================================================================

// Helper to check if we can afford a given cost
function canAffordCost(cost: ManaCost, state: GameState, forCreature?: CreatureCard): boolean {
  const maxMana = getMaxAvailableMana(state);

  // Quick check: do we have enough total mana?
  const totalCost =
    (cost.W || 0) +
    (cost.U || 0) +
    (cost.B || 0) +
    (cost.R || 0) +
    (cost.G || 0) +
    (cost.C || 0) +
    (cost.generic || 0);

  if (maxMana < totalCost) return false;

  // Check if we can produce each required color
  const colorCounts: ManaPool = emptyManaPool();

  for (const permanent of state.battlefield) {
    if (permanent.tapped) continue;
    if (permanent.card.type !== "land") continue;
    const colors = canTapForMana(permanent, state, forCreature);
    for (const color of colors) {
      colorCounts[color]++;
    }
  }

  // Check colored requirements
  if ((cost.W || 0) > 0 && colorCounts.W < (cost.W || 0)) return false;
  if ((cost.U || 0) > 0 && colorCounts.U < (cost.U || 0)) return false;
  if ((cost.B || 0) > 0 && colorCounts.B < (cost.B || 0)) return false;
  if ((cost.R || 0) > 0 && colorCounts.R < (cost.R || 0)) return false;
  if ((cost.G || 0) > 0 && colorCounts.G < (cost.G || 0)) return false;

  return true;
}

function canCastSpell(card: Card, state: GameState): boolean {
  if (card.type === "land") return false;
  if (!card.manaCost) return false;

  const forCreature = card.type === "creature" ? (card as CreatureCard) : undefined;

  // For creatures with impending, check if we can cast for impending cost
  if (forCreature?.impendingCost) {
    if (canAffordCost(forCreature.impendingCost, state, forCreature)) {
      return true;
    }
  }

  // Check regular mana cost
  return canAffordCost(card.manaCost, state, forCreature);
}

function tapLandsForCost(cost: ManaCost, state: GameState, forCreature?: CreatureCard): boolean {
  // Tap lands to pay the cost
  // Strategy: Use lands that only produce the required colors first

  const untappedLands = state.battlefield.filter(
    (p) => p.card.type === "land" && !p.tapped
  );

  // Pay colored costs first
  const colorsToPay: { color: ManaColor; amount: number }[] = [
    { color: "W", amount: cost.W || 0 },
    { color: "U", amount: cost.U || 0 },
    { color: "B", amount: cost.B || 0 },
    { color: "R", amount: cost.R || 0 },
    { color: "G", amount: cost.G || 0 },
    { color: "C", amount: cost.C || 0 },
  ];

  for (const { color, amount } of colorsToPay) {
    let remaining = amount;
    // Prefer lands that ONLY produce this color
    const singleColorLands = untappedLands.filter((p) => {
      if (p.tapped) return false;
      const colors = canTapForMana(p, state, forCreature);
      return colors.length === 1 && colors[0] === color;
    });

    for (const land of singleColorLands) {
      if (remaining <= 0) break;
      if (land.tapped) continue;
      tapLandForMana(land, color, state, forCreature);
      remaining--;
    }

    // Then use multi-color lands
    if (remaining > 0) {
      for (const land of untappedLands) {
        if (remaining <= 0) break;
        if (land.tapped) continue;
        const colors = canTapForMana(land, state, forCreature);
        if (colors.includes(color)) {
          tapLandForMana(land, color, state, forCreature);
          remaining--;
        }
      }
    }

    if (remaining > 0) return false; // Couldn't pay
  }

  // Pay generic with remaining untapped lands
  let genericRemaining = cost.generic || 0;
  for (const land of untappedLands) {
    if (genericRemaining <= 0) break;
    if (land.tapped) continue;
    const colors = canTapForMana(land, state, forCreature);
    if (colors.length > 0) {
      tapLandForMana(land, colors[0], state, forCreature);
      genericRemaining--;
    }
  }

  // Now pay the actual cost from pool
  return payManaCost(cost, state);
}

function castSpell(state: GameState, card: Card): boolean {
  if (!canCastSpell(card, state)) return false;
  if (!card.manaCost) return false;

  const handIndex = state.hand.findIndex((c) => c === card);
  if (handIndex === -1) return false;

  const forCreature = card.type === "creature" ? (card as CreatureCard) : undefined;

  // Determine if we should cast for impending cost
  let useImpending = false;
  let costToPay = card.manaCost;
  if (forCreature?.impendingCost) {
    // Prefer impending if we can't afford regular cost OR if impending is good strategy
    // For Overlord, always use impending when possible - it's cheaper and the mill triggers immediately
    if (!canAffordCost(card.manaCost, state, forCreature)) {
      // Can't afford regular cost, must use impending
      useImpending = true;
      costToPay = forCreature.impendingCost;
    } else if (canAffordCost(forCreature.impendingCost, state, forCreature)) {
      // Can afford both - prefer impending for Overlord (faster, mill happens immediately)
      useImpending = true;
      costToPay = forCreature.impendingCost;
    }
  }

  // Tap lands and pay cost
  if (!tapLandsForCost(costToPay, state, forCreature)) return false;

  // Remove from hand
  state.hand.splice(handIndex, 1);

  if (VERBOSE) {
    if (useImpending) {
      console.log(`  [Cast] ${card.name} (impending)`);
    } else {
      console.log(`  [Cast] ${card.name}`);
    }
  }

  // Handle by card type
  if (card.type === "creature") {
    const permanent: BattlefieldPermanent = {
      card,
      tapped: false,
      turnEntered: state.turn,
      counters: useImpending && forCreature?.impendingCounters ? { time: forCreature.impendingCounters } : undefined,
    };
    state.battlefield.push(permanent);

    // Resolve creature ETB triggers (impending creatures with counters also trigger on enter)
    resolveCreatureETB(state, permanent);
  } else if (card.type === "enchantment") {
    const permanent: BattlefieldPermanent = {
      card,
      tapped: false,
      turnEntered: state.turn,
    };
    state.battlefield.push(permanent);

    // Resolve enchantment ETB (Dredger's Insight)
    if (card.name === "Dredger's Insight") {
      resolveSpellAbility(state, card);
    }
  } else if (card.type === "saga") {
    const permanent: BattlefieldPermanent = {
      card,
      tapped: false,
      turnEntered: state.turn,
      counters: { time: 1 }, // Start with 1 lore counter
    };
    state.battlefield.push(permanent);

    // Resolve chapter 1 immediately
    resolveSagaChapter(state, permanent, 1);
  } else if (card.type === "instant" || card.type === "sorcery") {
    // Resolve spell ability first
    resolveSpellAbility(state, card);
    // Goes to graveyard after resolution
    state.graveyard.push(card);
  }

  return true;
}

// ============================================================================
// PHASE 2: COMBAT
// ============================================================================

function getEligibleAttackers(state: GameState): BattlefieldPermanent[] {
  return state.battlefield.filter((p) => {
    if (p.card.type !== "creature") return false;
    // Check summoning sickness
    if (p.turnEntered >= state.turn) return false;
    // Check if it's a creature (not impending with counters)
    if (p.counters?.time && p.counters.time > 0) return false;
    return !p.tapped;
  });
}

function declareAttackers(state: GameState): BattlefieldPermanent[] {
  // For now, attack with everything eligible
  const attackers = getEligibleAttackers(state);
  for (const attacker of attackers) {
    attacker.tapped = true;
  }
  return attackers;
}

function dealCombatDamage(
  state: GameState,
  attackers: BattlefieldPermanent[]
): number {
  let totalDamage = 0;
  for (const attacker of attackers) {
    const creature = attacker.card as CreatureCard;
    let power = creature.power;

    // Handle Spider-Man copying something
    if (attacker.isCopyOf) {
      const copiedCard = CARD_DATABASE[attacker.isCopyOf];
      if (copiedCard && copiedCard.type === "creature") {
        // Spider-Man is always 4/4 when copying
        power = 4;
      }
    }

    totalDamage += power;
  }

  state.opponentLife -= totalDamage;
  return totalDamage;
}

// ============================================================================
// PHASE 2: TURN STRUCTURE
// ============================================================================

function untapStep(state: GameState): void {
  for (const permanent of state.battlefield) {
    permanent.tapped = false;
  }
}

function drawStep(state: GameState): void {
  // Skip draw on turn 1 if on the play
  if (state.turn === 1 && state.onThePlay) return;
  drawCards(state, 1);

  // Add lore counters to sagas and resolve chapters
  for (const permanent of state.battlefield) {
    if (permanent.card.type === "saga") {
      // Add a lore counter (after draw step)
      if (!permanent.counters) permanent.counters = { time: 0 };
      const currentLore = permanent.counters.time || 0;
      // Only add if it's not the turn we cast it (already at 1 from ETB)
      if (permanent.turnEntered < state.turn) {
        permanent.counters.time = currentLore + 1;
        // Resolve the new chapter
        resolveSagaChapter(state, permanent, currentLore + 1);
      }
    }
  }
}

// Spells that can find lands when they mill
const LAND_FINDING_SPELLS = [
  "Cache Grab", // mill 4, return permanent (including land)
  "Dredger's Insight", // mill 4, return artifact/creature/land
  "Town Greeter", // mill 4, return land
];

function mainPhase(state: GameState): void {
  // SPECIAL CASE: Turn 4 combo check
  // If we have Spider-Man in hand, Bringer in GY, and can get to 4 mana by playing a land,
  // play the land FIRST before casting any other spells!
  const hasSpiderMan = state.hand.some(c => c.name === 'Superior Spider-Man');
  const hasBringerInGY = state.graveyard.some(c => c.name === 'Bringer of the Last Gift');
  const currentMana = getMaxAvailableMana(state);

  if (hasSpiderMan && hasBringerInGY && currentMana === 3 && !state.landPlayedThisTurn) {
    // Check if we have an untapped land to play
    const untappedLand = state.hand.find(c => {
      if (c.type !== 'land') return false;
      return willLandEnterUntapped(c as LandCard, state);
    });

    if (untappedLand) {
      // Play the land FIRST to enable turn 4 combo
      playLand(state, untappedLand);
      if (VERBOSE) {
        console.log(`  [COMBO SETUP] Played ${untappedLand.name} first to enable turn 4 combo`);
      }
    }
  }

  // STEP 1: If we haven't played a land yet and have land-finding spells,
  // cast those FIRST to potentially find a better land
  // BUT: If we have Bringer/Terror in hand and can cast Kiora, skip this step!
  // Kiora is more important (discards Bringer to graveyard for the combo)
  const hasBringerOrTerrorInHand = state.hand.some(
    c => c.name === "Bringer of the Last Gift" || c.name === "Terror of the Peaks"
  );
  const kioraInHand = state.hand.find(c => c.name === "Kiora, the Rising Tide");

  // Check if we can cast Kiora now OR if we could cast it after playing an untapped land
  const couldCastKioraAfterLandDrop = (): boolean => {
    if (!kioraInHand) return false;

    // Can cast now?
    if (canCastSpell(kioraInHand, state)) return true;

    // If we've already played a land, no look-ahead needed
    if (state.landPlayedThisTurn) return false;

    // Check if playing an untapped land would enable Kiora
    const currentMana = getMaxAvailableMana(state);
    const kioraCost = kioraInHand.manaValue || 3;

    // Would one more mana be enough?
    if (currentMana + 1 < kioraCost) return false;

    // Check if we have an untapped land that produces U (Kiora needs U)
    const hasUntappedLandWithU = state.hand.some(c => {
      if (c.type !== 'land') return false;
      const land = c as LandCard;
      if (!willLandEnterUntapped(land, state)) return false;
      // Check if land produces U
      return land.colors.includes('U');
    });

    // Also check if we already have U available and just need an untapped land for mana count
    const hasUAvailable = state.battlefield.some(p => {
      if (p.card.type !== 'land' || p.tapped) return false;
      const colors = canTapForMana(p, state);
      return colors.includes('U');
    });

    if (hasUAvailable) {
      // We have U, just need any untapped land for the mana count
      const hasAnyUntappedLand = state.hand.some(c => {
        if (c.type !== 'land') return false;
        return willLandEnterUntapped(c as LandCard, state);
      });
      return hasAnyUntappedLand;
    }

    // We need the new land to provide U
    return hasUntappedLandWithU;
  };

  const shouldPrioritizeKiora = hasBringerOrTerrorInHand && kioraInHand && couldCastKioraAfterLandDrop();

  if (!state.landPlayedThisTurn && !shouldPrioritizeKiora) {
    let foundLandFromMill = false;
    let castAny = true;

    while (castAny && !state.landPlayedThisTurn) {
      castAny = false;

      // Only look for land-finding spells we can cast
      const landFinders = state.hand.filter(
        (c) =>
          LAND_FINDING_SPELLS.includes(c.name) && canCastSpell(c, state)
      );

      if (landFinders.length > 0) {
        // Prefer cheaper spells first
        landFinders.sort((a, b) => (a.manaValue || 0) - (b.manaValue || 0));
        const spell = landFinders[0];
        const landsBeforeCast = state.hand.filter((c) => c.type === "land").length;

        if (castSpell(state, spell)) {
          castAny = true;
          // Check if we found a land
          const landsAfterCast = state.hand.filter((c) => c.type === "land").length;
          if (landsAfterCast > landsBeforeCast) {
            foundLandFromMill = true;
          }
        }
      } else {
        break; // No more land-finding spells
      }
    }
  }

  // STEP 2: Now play a land (possibly one we just found from milling)
  const landsInHand = state.hand.filter((c) => c.type === "land");
  if (landsInHand.length > 0 && !state.landPlayedThisTurn) {
    // Check what we could cast if we play an untapped land
    // Count current lands on battlefield (will untap next turn)
    const currentLandCount = state.battlefield.filter(p => p.card.type === "land").length;
    const manaAfterLandDrop = currentLandCount + 1;

    // Get spells and determine colors they need
    const spellsInHand = state.hand.filter(c => c.type !== "land");

    // Determine what colors we currently have access to (from lands on battlefield)
    const colorsAvailable = new Set<string>();
    for (const p of state.battlefield) {
      if (p.card.type === 'land') {
        const landColors = canTapForMana(p, state);
        for (const c of landColors) colorsAvailable.add(c);
      }
    }

    // Determine what colors we NEED for spells in hand
    const colorsNeeded = new Set<string>();
    for (const spell of spellsInHand) {
      const cost = (spell as any).manaCost || (spell as any).impendingCost;
      if (cost) {
        if (cost.W) colorsNeeded.add('W');
        if (cost.U) colorsNeeded.add('U');
        if (cost.B) colorsNeeded.add('B');
        if (cost.R) colorsNeeded.add('R');
        if (cost.G) colorsNeeded.add('G');
      }
    }

    // Find colors we need but don't have
    const missingColors = new Set<string>();
    for (const c of colorsNeeded) {
      if (!colorsAvailable.has(c)) missingColors.add(c);
    }

    // Helper to determine if a land enters tapped
    const entersT = (l: LandCard): boolean => {
      return !willLandEnterUntapped(l, state);
    };

    // Helper to check if a land provides a missing color
    const providesMissingColor = (l: LandCard): boolean => {
      for (const c of l.colors) {
        if (missingColors.has(c)) return true;
      }
      return false;
    };

    // Check if we can ACTUALLY cast something this turn
    // This considers both mana count AND colors
    const canCastSomethingThisTurn = (potentialLand: LandCard): boolean => {
      // If land enters tapped, we can't use it this turn
      if (entersT(potentialLand)) return false;

      // What colors would we have after playing this land?
      const colorsAfter = new Set(colorsAvailable);
      for (const c of potentialLand.colors) colorsAfter.add(c);

      // Can we cast any spell?
      return spellsInHand.some(spell => {
        const mv = (spell as any).manaValue || 0;
        if (mv > manaAfterLandDrop) return false;

        // Check color requirements
        const cost = (spell as any).manaCost || (spell as any).impendingCost;
        if (!cost) return true;
        if (cost.U && !colorsAfter.has('U')) return false;
        if (cost.B && !colorsAfter.has('B')) return false;
        if (cost.G && !colorsAfter.has('G')) return false;
        if (cost.W && !colorsAfter.has('W')) return false;
        if (cost.R && !colorsAfter.has('R')) return false;
        return true;
      });
    };

    // Sort lands by priority
    const sorted = [...landsInHand].sort((a, b) => {
      const aLand = a as LandCard;
      const bLand = b as LandCard;

      const aTapped = entersT(aLand);
      const bTapped = entersT(bLand);

      const aProvidesMissing = providesMissingColor(aLand);
      const bProvidesMissing = providesMissingColor(bLand);

      const aEnablesCast = canCastSomethingThisTurn(aLand);
      const bEnablesCast = canCastSomethingThisTurn(bLand);

      // PRIORITY 0: Lands that ENABLE casting something this turn are preferred
      // (This means: untapped + provides needed colors)
      if (aEnablesCast !== bEnablesCast) {
        return aEnablesCast ? -1 : 1;
      }

      // PRIORITY 1: If neither enables casting, prefer lands that provide missing colors
      // This ensures we get the right colors for future turns
      if (!aEnablesCast && !bEnablesCast) {
        if (aProvidesMissing !== bProvidesMissing) {
          return aProvidesMissing ? -1 : 1;
        }
        // Prefer surveil tapped lands (get value!)
        if (aLand.hasSurveil !== bLand.hasSurveil) {
          return aLand.hasSurveil ? -1 : 1;
        }
        // Prefer tapped (save untapped for later)
        if (aTapped !== bTapped) {
          return aTapped ? -1 : 1;
        }
        return 0;
      }

      // PRIORITY 2: Both enable casting - prefer the one that provides more colors
      // or surveil for value
      if (aLand.hasSurveil !== bLand.hasSurveil) {
        return aLand.hasSurveil ? -1 : 1;
      }
      return bLand.colors.length - aLand.colors.length;
    });
    playLand(state, sorted[0]);
  }

  // STEP 3: Cast remaining spells
  let castAny = true;
  while (castAny) {
    castAny = false;

    // Get castable spells, sorted by priority
    // Check game state for spell priorities
    const hasBringerInGraveyard = state.graveyard.some(
      (c) => c.name === "Bringer of the Last Gift"
    );
    const hasBringerInHand = state.hand.some(
      (c) => c.name === "Bringer of the Last Gift"
    );
    const hasTerrorInHand = state.hand.some(
      (c) => c.name === "Terror of the Peaks"
    );

    // Check if the combo would be lethal
    const comboIsLethal = hasBringerInGraveyard && isComboLethal(state);
    const hasSpiderManInHand = state.hand.some((c) => c.name === "Superior Spider-Man");

    // Log when we're holding back the combo
    if (VERBOSE && hasBringerInGraveyard && hasSpiderManInHand && !comboIsLethal) {
      const expectedDamage = calculateComboDamage(state);
      console.log(
        `  [Waiting] Combo not lethal yet (expected: ${expectedDamage} damage, need: ${state.opponentLife})`
      );
    }

    const castableSpells = state.hand
      .filter((c) => {
        if (c.type === "land") return false;
        if (!canCastSpell(c, state)) return false;

        // Only cast Spider-Man if the combo would be LETHAL
        // (expected damage >= opponent's life)
        if (c.name === "Superior Spider-Man") {
          if (!hasBringerInGraveyard) return false; // Need Bringer in graveyard
          if (!comboIsLethal) return false; // Wait until it would kill
        }

        return true;
      })
      .sort((a, b) => {
        // Priority order for this combo deck:
        // 1. Superior Spider-Man copying Bringer (THE COMBO) - only when lethal
        // 2. Kiora if we have Bringer in hand (to discard it)
        // 3. Mill spells (Cache Grab, Dredger's Insight, Town Greeter, Overlord)
        // 4. Awaken the Honored Dead (saga that mills later)
        // 5. Other spells by mana cost

        // If combo is lethal, Spider-Man is #1 priority (GO FOR THE WIN!)
        if (comboIsLethal) {
          if (a.name === "Superior Spider-Man") return -1;
          if (b.name === "Superior Spider-Man") return 1;
        }

        // If Bringer in hand, Kiora is priority (to discard Bringer)
        // This should come BEFORE mill spell check!
        if (hasBringerInHand) {
          if (a.name === "Kiora, the Rising Tide") return -1;
          if (b.name === "Kiora, the Rising Tide") return 1;
        }

        // Also check if Terror is in hand - Kiora can discard that too
        if (hasTerrorInHand) {
          if (a.name === "Kiora, the Rising Tide") return -1;
          if (b.name === "Kiora, the Rising Tide") return 1;
        }

        // Mill spells are high priority to find combo pieces
        const millSpells = [
          "Cache Grab",
          "Dredger's Insight",
          "Town Greeter",
          "Overlord of the Balemurk",
        ];
        const aIsMill = millSpells.includes(a.name);
        const bIsMill = millSpells.includes(b.name);
        if (aIsMill && !bIsMill) return -1;
        if (bIsMill && !aIsMill) return 1;

        // Awaken the Honored Dead is a good fallback - it mills on chapter 2
        // and returns a creature on chapter 3
        if (a.name === "Awaken the Honored Dead" && !bIsMill) return -1;
        if (b.name === "Awaken the Honored Dead" && !aIsMill) return 1;

        // Prefer cheaper spells to cast more per turn
        return (a.manaValue || 0) - (b.manaValue || 0);
      });

    if (castableSpells.length > 0) {
      const spell = castableSpells[0];
      if (castSpell(state, spell)) {
        castAny = true;
      }
    }
  }
}

function combatPhase(state: GameState): number {
  const attackers = declareAttackers(state);
  if (attackers.length === 0) return 0;
  return dealCombatDamage(state, attackers);
}

function endStep(state: GameState): void {
  // Remove time counters from impending permanents
  for (const permanent of state.battlefield) {
    if (permanent.counters?.time && permanent.counters.time > 0) {
      permanent.counters.time--;
    }
  }

  // Saga lore counters (added at draw step, but check for sacrifice here)
  // Discard to hand size 7 if needed
  while (state.hand.length > 7) {
    // Discard worst card - prefer discarding Bringer/Terror
    const toDiscard = state.hand.find(
      (c) =>
        c.name === "Bringer of the Last Gift" ||
        c.name === "Terror of the Peaks"
    );
    if (toDiscard) {
      const idx = state.hand.indexOf(toDiscard);
      state.hand.splice(idx, 1);
      state.graveyard.push(toDiscard);
    } else {
      // Discard last card
      const card = state.hand.pop()!;
      state.graveyard.push(card);
    }
  }

  // Empty mana pool
  state.manaPool = emptyManaPool();
}

function playTurn(state: GameState): { combatDamage: number } {
  state.turn++;
  state.landPlayedThisTurn = false;
  state.phase = "untap";

  if (VERBOSE) {
    console.log(`\n=== TURN ${state.turn} ===`);
  }

  // Untap
  untapStep(state);

  // Draw
  state.phase = "draw";
  const handBefore = state.hand.length;
  drawStep(state);
  if (VERBOSE && state.hand.length > handBefore) {
    const drewCard = state.hand[state.hand.length - 1];
    console.log(`[Draw] Drew: ${drewCard.name}`);
  } else if (VERBOSE && state.turn === 1 && state.onThePlay) {
    console.log(`[Draw] Skipped (on the play)`);
  }

  // Main Phase 1
  state.phase = "main1";
  if (VERBOSE) {
    console.log(`[Main 1] Hand: ${state.hand.map(c => c.name).join(", ")}`);
  }
  mainPhase(state);

  // Combat
  state.phase = "combat";
  const combatDamage = combatPhase(state);
  if (VERBOSE && combatDamage > 0) {
    console.log(`[Combat] Dealt ${combatDamage} damage (opponent: ${state.opponentLife} life)`);
  }

  // Main Phase 2
  state.phase = "main2";
  // Additional spell casting could happen here

  // End Step
  state.phase = "end";
  endStep(state);

  if (VERBOSE) {
    console.log(`[End of Turn ${state.turn}]`);
    console.log(`  Battlefield: ${state.battlefield.map(p => {
      let name = p.card.name;
      if (p.isCopyOf) name += ` (copy of ${p.isCopyOf})`;
      if (p.counters?.time) name += ` (${p.counters.time} time counters)`;
      return name;
    }).join(", ") || "(empty)"}`);
    console.log(`  Graveyard: ${state.graveyard.map(c => c.name).join(", ") || "(empty)"}`);
    console.log(`  Opponent life: ${state.opponentLife}`);
  }

  return { combatDamage };
}

// ============================================================================
// PHASE 2: GAME LOOP
// ============================================================================

function initializeGame(): GameState {
  const state = createInitialGameState();
  state.library = shuffle(buildDeck());
  selectOpeningHand(state);

  if (VERBOSE) {
    console.log(`=== Game Start (seed: ${getSeed()}, deck: ${DECK_FILE}) ===`);
    console.log(`On the ${state.onThePlay ? "play" : "draw"}`);
    console.log(`Opening hand (${state.hand.length} cards):`);
    for (const card of state.hand) {
      console.log(`  - ${card.name}`);
    }
    console.log(``);
  }

  return state;
}

function checkWinCondition(state: GameState): boolean {
  return state.opponentLife <= 0;
}

// Global flag for verbose output
let VERBOSE = false;

// Helper to get available colors from battlefield
function getAvailableColors(state: GameState): Set<string> {
  const colors = new Set<string>();
  for (const p of state.battlefield) {
    if (p.card.type === 'land') {
      const landColors = canTapForMana(p, state);
      for (const c of landColors) colors.add(c);
    }
  }
  return colors;
}

function runGame(): {
  winTurn: number;
  onThePlay: boolean;
  totalCombatDamage: number;
  comboDamage: number;
  turnWithU: number;
  turnWithB: number;
  turnWithG: number;
  turnWithUBG: number;
} {
  const state = initializeGame();
  let totalCombatDamage = 0;
  let comboDamage = 0;
  const maxTurns = 20;

  // Track when each color becomes available
  let turnWithU = -1;
  let turnWithB = -1;
  let turnWithG = -1;
  let turnWithUBG = -1;

  while (state.turn < maxTurns && !checkWinCondition(state)) {
    const lifeBefore = state.opponentLife;
    const result = playTurn(state);
    totalCombatDamage += result.combatDamage;

    // Track combo damage (non-combat damage dealt this turn)
    const totalDamageThisTurn = lifeBefore - state.opponentLife;
    comboDamage += totalDamageThisTurn - result.combatDamage;

    // Check what colors are available after this turn
    const colors = getAvailableColors(state);
    if (turnWithU === -1 && colors.has('U')) turnWithU = state.turn;
    if (turnWithB === -1 && colors.has('B')) turnWithB = state.turn;
    if (turnWithG === -1 && colors.has('G')) turnWithG = state.turn;
    if (turnWithUBG === -1 && colors.has('U') && colors.has('B') && colors.has('G')) {
      turnWithUBG = state.turn;
    }
  }

  if (VERBOSE) {
    console.log(`\n=== GAME OVER ===`);
    console.log(`Won on turn: ${checkWinCondition(state) ? state.turn : "Did not win"}`);
  }

  return {
    winTurn: checkWinCondition(state) ? state.turn : -1,
    onThePlay: state.onThePlay,
    totalCombatDamage,
    comboDamage,
    turnWithU,
    turnWithB,
    turnWithG,
    turnWithUBG,
  };
}

// ============================================================================
// TEST: Run multiple games and collect statistics
// ============================================================================

// Function to run simulation and return statistics
function runSimulation(
  numGames: number,
  deckFile: string
): {
  avgWinTurn: number;
  winRate: number;
  turnDist: Record<number, number>;
  minWinTurn: number;
  maxWinTurn: number;
  avgTurnWithU: number;
  avgTurnWithB: number;
  avgTurnWithG: number;
  avgTurnWithUBG: number;
} {
  const savedDeckFile = DECK_FILE;
  DECK_FILE = deckFile;

  const results: {
    winTurn: number;
    turnWithU: number;
    turnWithB: number;
    turnWithG: number;
    turnWithUBG: number;
  }[] = [];
  for (let i = 0; i < numGames; i++) {
    // Reset seed for each game to ensure independent randomness
    setSeed(Math.floor(Math.random() * 2147483647));
    const result = runGame();
    results.push({
      winTurn: result.winTurn,
      turnWithU: result.turnWithU,
      turnWithB: result.turnWithB,
      turnWithG: result.turnWithG,
      turnWithUBG: result.turnWithUBG,
    });
  }

  DECK_FILE = savedDeckFile;

  const wins = results.filter((r) => r.winTurn > 0);
  const winTurns = wins.map((r) => r.winTurn);
  const avgWinTurn = winTurns.length > 0 ? winTurns.reduce((a, b) => a + b, 0) / winTurns.length : -1;
  const turnDist: Record<number, number> = {};
  for (const turn of winTurns) {
    turnDist[turn] = (turnDist[turn] || 0) + 1;
  }

  // Calculate average turn for mana color availability
  const validU = results.filter(r => r.turnWithU > 0).map(r => r.turnWithU);
  const validB = results.filter(r => r.turnWithB > 0).map(r => r.turnWithB);
  const validG = results.filter(r => r.turnWithG > 0).map(r => r.turnWithG);
  const validUBG = results.filter(r => r.turnWithUBG > 0).map(r => r.turnWithUBG);

  const avgTurnWithU = validU.length > 0 ? validU.reduce((a, b) => a + b, 0) / validU.length : -1;
  const avgTurnWithB = validB.length > 0 ? validB.reduce((a, b) => a + b, 0) / validB.length : -1;
  const avgTurnWithG = validG.length > 0 ? validG.reduce((a, b) => a + b, 0) / validG.length : -1;
  const avgTurnWithUBG = validUBG.length > 0 ? validUBG.reduce((a, b) => a + b, 0) / validUBG.length : -1;

  return {
    avgWinTurn,
    winRate: wins.length / numGames,
    turnDist,
    minWinTurn: winTurns.length > 0 ? Math.min(...winTurns) : -1,
    maxWinTurn: winTurns.length > 0 ? Math.max(...winTurns) : -1,
    avgTurnWithU,
    avgTurnWithB,
    avgTurnWithG,
    avgTurnWithUBG,
  };
}

// Check mode
const isCompareMode = process.argv.length >= 5 && process.argv[2] === "compare";
const isOptimizeMode = process.argv[2] === "optimize";

// ============================================================================
// OPTIMIZE MODE - Find best land configuration
// ============================================================================

// List of all land types and their constraints
// Basic lands have no max limit, non-basic are limited to 4
const LAND_TYPES = [
  { name: "Forest", min: 0, max: 4, isBasic: true },
  { name: "Island", min: 0, max: 4, isBasic: true },
  { name: "Swamp", min: 0, max: 4, isBasic: true },
  { name: "Watery Grave", min: 0, max: 4, isBasic: false },
  { name: "Undercity Sewers", min: 0, max: 4, isBasic: false },
  { name: "Underground Mortuary", min: 0, max: 4, isBasic: false },
  { name: "Cavern of Souls", min: 0, max: 4, isBasic: false },
  { name: "Restless Cottage", min: 0, max: 1, isBasic: false },
  { name: "Wastewood Verge", min: 0, max: 4, isBasic: false },
  { name: "Gloomlake Verge", min: 0, max: 4, isBasic: false },
  { name: "Multiversal Passage", min: 0, max: 4, isBasic: false },
  { name: "Blooming Marsh", min: 0, max: 4, isBasic: false },
  { name: "Starting Town", min: 0, max: 4, isBasic: false },
];

// Non-land cards that stay fixed
const FIXED_CARDS = [
  { name: "Terror of the Peaks", count: 4 },
  { name: "Bringer of the Last Gift", count: 4 },
  { name: "Superior Spider-Man", count: 4 },
  { name: "Overlord of the Balemurk", count: 4 },
  { name: "Kiora, the Rising Tide", count: 4 },
  { name: "Town Greeter", count: 3 },
  { name: "Cache Grab", count: 4 },
  { name: "Dredger's Insight", count: 4 },
  { name: "Awaken the Honored Dead", count: 4 },
  { name: "Analyze the Pollen", count: 1 },
];

const TOTAL_FIXED = FIXED_CARDS.reduce((sum, c) => sum + c.count, 0); // 36 non-lands
const TOTAL_LANDS = 60 - TOTAL_FIXED; // 24 lands

interface LandConfig {
  [landName: string]: number;
}

// Strategy 1: Weighted random allocation (original)
// Randomly assign counts to each land type, respecting max limits
function generateRandomLandConfigWeighted(): LandConfig {
  const config: LandConfig = {};
  let remaining = TOTAL_LANDS;

  // Shuffle all land types randomly
  const shuffledLands = [...LAND_TYPES].sort(() => random() - 0.5);

  // First pass: assign random counts respecting max limits
  for (const land of shuffledLands) {
    const maxAllowed = Math.min(land.max, remaining);
    config[land.name] = Math.floor(random() * (maxAllowed + 1));
    remaining -= config[land.name];
  }

  // Second pass: if we still have remaining slots, distribute them
  // Keep adding to random lands until we hit 24
  let attempts = 0;
  while (remaining > 0 && attempts < 1000) {
    const randomLand = shuffledLands[Math.floor(random() * shuffledLands.length)];
    if (config[randomLand.name] < randomLand.max) {
      config[randomLand.name]++;
      remaining--;
    }
    attempts++;
  }

  // If still not at 24 (shouldn't happen with enough land types), log warning
  if (remaining > 0) {
    console.warn(`Warning: Could not allocate all ${TOTAL_LANDS} lands, ${remaining} remaining`);
  }

  return config;
}

// Strategy 2: Shuffle and deal
// Put max copies of each land in a pool, shuffle, take first 24
function generateRandomLandConfigShuffle(): LandConfig {
  const config: LandConfig = {};

  // Create a pool with max copies of each land
  const pool: string[] = [];
  for (const landType of LAND_TYPES) {
    for (let i = 0; i < landType.max; i++) {
      pool.push(landType.name);
    }
  }

  // Shuffle the pool (Fisher-Yates)
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }

  // Take the first 24
  for (let i = 0; i < TOTAL_LANDS; i++) {
    const landName = pool[i];
    config[landName] = (config[landName] || 0) + 1;
  }

  return config;
}

// Generate a random land configuration using the specified strategy
type LandStrategy = "weighted" | "shuffle";
function generateRandomLandConfig(strategy: LandStrategy = "weighted"): LandConfig {
  if (strategy === "shuffle") {
    return generateRandomLandConfigShuffle();
  }
  return generateRandomLandConfigWeighted();
}

function buildDeckFromConfig(config: LandConfig): Card[] {
  const deck: Card[] = [];

  // Add fixed cards
  for (const { name, count } of FIXED_CARDS) {
    const cardDef = CARD_DATABASE[name];
    if (cardDef) {
      for (let i = 0; i < count; i++) {
        deck.push({ ...cardDef });
      }
    }
  }

  // Add lands from config
  for (const [landName, count] of Object.entries(config)) {
    const cardDef = CARD_DATABASE[landName];
    if (cardDef && count > 0) {
      for (let i = 0; i < count; i++) {
        deck.push({ ...cardDef });
      }
    }
  }

  return deck;
}

function runSimulationWithConfig(config: LandConfig, numGames: number): { avgWinTurn: number; winRate: number } {
  const results: number[] = [];

  for (let i = 0; i < numGames; i++) {
    setSeed(Math.floor(Math.random() * 2147483647));

    const state = createInitialGameState();
    state.library = shuffle(buildDeckFromConfig(config));
    selectOpeningHand(state);

    let winTurn = -1;
    for (let turn = 1; turn <= 20 && state.opponentLife > 0; turn++) {
      playTurn(state);
      if (state.opponentLife <= 0) {
        winTurn = state.turn;
        break;
      }
    }

    results.push(winTurn);
  }

  const wins = results.filter((t) => t > 0);
  const avgWinTurn = wins.length > 0 ? wins.reduce((a, b) => a + b, 0) / wins.length : -1;
  const winRate = wins.length / numGames;

  return { avgWinTurn, winRate };
}

function configToString(config: LandConfig): string {
  return Object.entries(config)
    .filter(([_, count]) => count > 0)
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([name, count]) => `${count} ${name}`)
    .join(", ");
}

if (isOptimizeMode) {
  // Optimize mode: bun run simulator.ts optimize [numConfigs] [gamesPerConfig] [strategy]
  // Strategy can be "weighted" (default) or "shuffle"
  const numConfigs = process.argv[3] ? parseInt(process.argv[3], 10) : 1000;
  const gamesPerConfig = process.argv[4] ? parseInt(process.argv[4], 10) : 1000;
  const strategy: LandStrategy = (process.argv[5] === "shuffle") ? "shuffle" : "weighted";

  console.log("=== MTG Land Optimization ===\n");
  console.log(`Strategy: ${strategy}`);
  console.log(`  - weighted: Random counts for each land type, respecting max limits`);
  console.log(`  - shuffle:  Put max copies of each land in pool, shuffle, take 24\n`);
  console.log(`Testing ${numConfigs} random land configurations`);
  console.log(`Running ${gamesPerConfig} games per configuration...\n`);
  console.log(`Fixed non-land cards: ${TOTAL_FIXED} cards`);
  console.log(`Land slots to fill: ${TOTAL_LANDS} cards\n`);

  let bestConfig: LandConfig | null = null;
  let bestAvgTurn = Infinity;
  let bestWinRate = 0;

  const allResults: { config: LandConfig; avgWinTurn: number; winRate: number }[] = [];
  const startTime = Date.now();

  for (let i = 0; i < numConfigs; i++) {
    const config = generateRandomLandConfig(strategy);
    const result = runSimulationWithConfig(config, gamesPerConfig);

    allResults.push({ config, ...result });

    if (result.avgWinTurn > 0 && result.avgWinTurn < bestAvgTurn) {
      bestConfig = config;
      bestAvgTurn = result.avgWinTurn;
      bestWinRate = result.winRate;

      console.log(`[${i + 1}/${numConfigs}] New best! Avg turn: ${bestAvgTurn.toFixed(3)}, Win rate: ${(bestWinRate * 100).toFixed(1)}%`);
      console.log(`  Lands: ${configToString(config)}\n`);
    }

    // Progress update every 100 configs
    if ((i + 1) % 100 === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      const eta = (elapsed / (i + 1)) * (numConfigs - i - 1);
      console.log(`Progress: ${i + 1}/${numConfigs} (${((i + 1) / numConfigs * 100).toFixed(1)}%) - ETA: ${eta.toFixed(0)}s`);
    }
  }

  const totalTime = (Date.now() - startTime) / 1000;

  console.log(`\n=== Optimization Complete ===`);
  console.log(`Total time: ${totalTime.toFixed(1)}s`);
  console.log(`Configurations tested: ${numConfigs}`);
  console.log(`Games per config: ${gamesPerConfig}`);
  console.log(`Total games: ${(numConfigs * gamesPerConfig).toLocaleString()}\n`);

  console.log(`=== BEST LAND CONFIGURATION ===`);
  console.log(`Average win turn: ${bestAvgTurn.toFixed(3)}`);
  console.log(`Win rate: ${(bestWinRate * 100).toFixed(1)}%`);
  console.log(`\nLand breakdown:`);
  if (bestConfig) {
    for (const land of LAND_TYPES) {
      const count = bestConfig[land.name] || 0;
      if (count > 0) {
        console.log(`  ${count} ${land.name}`);
      }
    }
  }

  // Show top 10 configurations
  console.log(`\n=== Top 10 Configurations ===`);
  allResults.sort((a, b) => a.avgWinTurn - b.avgWinTurn);
  for (let i = 0; i < Math.min(10, allResults.length); i++) {
    const r = allResults[i];
    console.log(`${i + 1}. Avg: ${r.avgWinTurn.toFixed(3)}, WR: ${(r.winRate * 100).toFixed(1)}%`);
    console.log(`   ${configToString(r.config)}\n`);
  }

  // Analyze what lands appear most in top configs
  console.log(`\n=== Land Frequency in Top 50 Configs ===`);
  const top50 = allResults.slice(0, Math.min(50, allResults.length));
  const landTotals: Record<string, { total: number; count: number }> = {};
  for (const r of top50) {
    for (const [landName, count] of Object.entries(r.config)) {
      if (!landTotals[landName]) {
        landTotals[landName] = { total: 0, count: 0 };
      }
      landTotals[landName].total += count;
      if (count > 0) landTotals[landName].count++;
    }
  }

  const landAvgs = Object.entries(landTotals)
    .map(([name, data]) => ({ name, avg: data.total / top50.length, frequency: data.count / top50.length }))
    .sort((a, b) => b.avg - a.avg);

  console.log(`Land                    | Avg Count | In Top 50`);
  console.log(`------------------------|-----------|----------`);
  for (const { name, avg, frequency } of landAvgs) {
    console.log(`${name.padEnd(24)}| ${avg.toFixed(2).padStart(9)} | ${(frequency * 100).toFixed(0).padStart(7)}%`);
  }

} else if (isCompareMode) {
  // Compare mode: bun run simulator.ts compare deck1.txt deck2.txt [numGames]
  const deck1 = process.argv[3];
  const deck2 = process.argv[4];
  const numGames = process.argv[5] ? parseInt(process.argv[5], 10) : 1000;

  console.log("=== MTG Deck Comparison ===\n");
  console.log(`Comparing: ${deck1} vs ${deck2}`);
  console.log(`Running ${numGames} games each...\n`);

  const stats1 = runSimulation(numGames, deck1);
  const stats2 = runSimulation(numGames, deck2);

  console.log(`\n=== Results ===\n`);
  console.log(`Deck 1 (${deck1}):`);
  console.log(`  Win rate: ${(stats1.winRate * 100).toFixed(1)}%`);
  console.log(`  Average win turn: ${stats1.avgWinTurn.toFixed(2)}`);
  console.log(`  Fastest win: Turn ${stats1.minWinTurn}`);
  console.log(`  Slowest win: Turn ${stats1.maxWinTurn}`);

  console.log(`\nDeck 2 (${deck2}):`);
  console.log(`  Win rate: ${(stats2.winRate * 100).toFixed(1)}%`);
  console.log(`  Average win turn: ${stats2.avgWinTurn.toFixed(2)}`);
  console.log(`  Fastest win: Turn ${stats2.minWinTurn}`);
  console.log(`  Slowest win: Turn ${stats2.maxWinTurn}`);

  const diff = stats2.avgWinTurn - stats1.avgWinTurn;
  console.log(`\n=== Comparison ===`);
  if (diff > 0.05) {
    console.log(`${deck1} is FASTER by ${diff.toFixed(2)} turns on average`);
  } else if (diff < -0.05) {
    console.log(`${deck2} is FASTER by ${(-diff).toFixed(2)} turns on average`);
  } else {
    console.log(`Both decks perform similarly (difference: ${Math.abs(diff).toFixed(2)} turns)`);
  }

  // Show mana color availability
  console.log(`\n=== Mana Color Availability (avg turn) ===`);
  console.log(`Color | ${deck1.padEnd(10)} | ${deck2.padEnd(10)}`);
  console.log(`------|${"".padEnd(12, "-")}|${"".padEnd(12, "-")}`);
  console.log(`U     | ${stats1.avgTurnWithU.toFixed(2).padStart(10)} | ${stats2.avgTurnWithU.toFixed(2).padStart(10)}`);
  console.log(`B     | ${stats1.avgTurnWithB.toFixed(2).padStart(10)} | ${stats2.avgTurnWithB.toFixed(2).padStart(10)}`);
  console.log(`G     | ${stats1.avgTurnWithG.toFixed(2).padStart(10)} | ${stats2.avgTurnWithG.toFixed(2).padStart(10)}`);
  console.log(`U+B+G | ${stats1.avgTurnWithUBG.toFixed(2).padStart(10)} | ${stats2.avgTurnWithUBG.toFixed(2).padStart(10)}`);

  // Show turn distribution comparison
  console.log(`\n=== Turn Distribution ===`);
  const allTurns = new Set([...Object.keys(stats1.turnDist), ...Object.keys(stats2.turnDist)].map(Number));
  console.log(`Turn | ${deck1.padEnd(15)} | ${deck2.padEnd(15)}`);
  console.log(`-----|${"".padEnd(17, "-")}|${"".padEnd(17, "-")}`);
  for (const turn of [...allTurns].sort((a, b) => a - b)) {
    const count1 = stats1.turnDist[turn] || 0;
    const count2 = stats2.turnDist[turn] || 0;
    const pct1 = ((count1 / numGames) * 100).toFixed(1).padStart(5);
    const pct2 = ((count2 / numGames) * 100).toFixed(1).padStart(5);
    console.log(`  ${turn}  | ${pct1}% (${count1.toString().padStart(4)}) | ${pct2}% (${count2.toString().padStart(4)})`);
  }

} else {
  // Normal mode - run verbose game and statistics

  console.log("=== MTG Reanimator Simulator ===\n");

  // Run single game with verbose output
  console.log("--- Single Game (Verbose) ---\n");
  VERBOSE = true;
  const singleResult = runGame();
  VERBOSE = false;
  console.log(`\n=== Single Game Result ===`);
  console.log(`On the play: ${singleResult.onThePlay}`);
  console.log(`Win turn: ${singleResult.winTurn === -1 ? "Did not win" : singleResult.winTurn}`);
  console.log(`Combat damage: ${singleResult.totalCombatDamage}`);
  console.log(`Combo damage: ${singleResult.comboDamage}`);

  // Run multiple games for statistics
  console.log("\n--- Running 10000 games for statistics ---\n");

  const NUM_GAMES = 1000; // Quick stats, increase for more precision
  const results: { winTurn: number; onThePlay: boolean }[] = [];

for (let i = 0; i < NUM_GAMES; i++) {
  const result = runGame();
  results.push({ winTurn: result.winTurn, onThePlay: result.onThePlay });
}

// Calculate statistics
const wins = results.filter((r) => r.winTurn > 0);
const losses = results.filter((r) => r.winTurn === -1);
const winTurns = wins.map((r) => r.winTurn);
const avgWinTurn = winTurns.reduce((a, b) => a + b, 0) / winTurns.length;
const minWinTurn = Math.min(...winTurns);
const maxWinTurn = Math.max(...winTurns);

// Win rate by play/draw
const onPlayWins = wins.filter((r) => r.onThePlay).length;
const onPlayTotal = results.filter((r) => r.onThePlay).length;
const onDrawWins = wins.filter((r) => !r.onThePlay).length;
const onDrawTotal = results.filter((r) => !r.onThePlay).length;

// Turn distribution
const turnDist: Record<number, number> = {};
for (const turn of winTurns) {
  turnDist[turn] = (turnDist[turn] || 0) + 1;
}

console.log(`=== Statistics (${NUM_GAMES} games) ===`);
console.log(`Win rate: ${((wins.length / NUM_GAMES) * 100).toFixed(1)}%`);
console.log(`Average win turn: ${avgWinTurn.toFixed(2)}`);
console.log(`Fastest win: Turn ${minWinTurn}`);
console.log(`Slowest win: Turn ${maxWinTurn}`);
console.log(`\nOn the play: ${onPlayWins}/${onPlayTotal} wins (${((onPlayWins / onPlayTotal) * 100).toFixed(1)}%)`);
console.log(`On the draw: ${onDrawWins}/${onDrawTotal} wins (${((onDrawWins / onDrawTotal) * 100).toFixed(1)}%)`);
console.log(`\nWin turn distribution:`);
for (const turn of Object.keys(turnDist).map(Number).sort((a, b) => a - b)) {
  const count = turnDist[turn];
  const bar = "█".repeat(Math.ceil(count / 2));
  console.log(`  Turn ${turn}: ${bar} (${count})`);
}
console.log(`\nPhase 3 complete!`);

// Diagnostic: Run games to find turn 4 win potential
console.log(`\n--- Turn 4 Win Analysis (1000 games with full simulation) ---`);
let turn4Possible = 0;
let turn4Reasons: Record<string, number> = {};

for (let i = 0; i < 1000; i++) {
  const state = createInitialGameState();
  state.library = shuffle(buildDeck());
  selectOpeningHand(state);

  // Simulate turns 1-4 properly with untapping
  for (let t = 1; t <= 4; t++) {
    state.turn = t;
    state.landPlayedThisTurn = false;

    // UNTAP all permanents at start of turn!
    for (const p of state.battlefield) {
      p.tapped = false;
    }

    // Draw (skip turn 1 on play)
    if (t > 1 || !state.onThePlay) {
      if (state.library.length > 0) {
        state.hand.push(state.library.shift()!);
      }
    }

    // Count mana from untapped lands
    let untappedMana = state.battlefield.filter(p => p.card.type === 'land' && !p.tapped).length;

    // Cast 2-mana mill spells first (Cache Grab, Dredger's Insight)
    if (untappedMana >= 2) {
      const millSpell = state.hand.find(c => c.name === "Dredger's Insight" || c.name === 'Cache Grab');
      if (millSpell) {
        state.hand = state.hand.filter(c => c !== millSpell);
        untappedMana -= 2;
        // Mill 4
        for (let m = 0; m < 4 && state.library.length > 0; m++) {
          state.graveyard.push(state.library.shift()!);
        }
        // Dredger's can return Spider-Man
        if (millSpell.name === "Dredger's Insight") {
          const spiderIdx = state.graveyard.findIndex(c => c.name === 'Superior Spider-Man');
          if (spiderIdx !== -1 && !state.hand.some(c => c.name === 'Superior Spider-Man')) {
            state.hand.push(state.graveyard.splice(spiderIdx, 1)[0]);
          }
        }
      }
    }

    // Cast Kiora (3 mana) if we have Bringer in hand to discard
    if (untappedMana >= 3) {
      const kiora = state.hand.find(c => c.name === 'Kiora, the Rising Tide');
      const bringerInHand = state.hand.find(c => c.name === 'Bringer of the Last Gift');
      if (kiora && bringerInHand) {
        state.hand = state.hand.filter(c => c !== kiora);
        untappedMana -= 3;
        // Draw 2
        for (let d = 0; d < 2 && state.library.length > 0; d++) {
          state.hand.push(state.library.shift()!);
        }
        // Discard Bringer and Terror if possible
        const bIdx = state.hand.findIndex(c => c.name === 'Bringer of the Last Gift');
        if (bIdx !== -1) state.graveyard.push(state.hand.splice(bIdx, 1)[0]);
        const tIdx = state.hand.findIndex(c => c.name === 'Terror of the Peaks');
        if (tIdx !== -1) state.graveyard.push(state.hand.splice(tIdx, 1)[0]);
        else if (state.hand.length > 0) state.graveyard.push(state.hand.splice(0, 1)[0]); // Discard anything
      }
    }

    // Play land - prioritize untapped
    if (!state.landPlayedThisTurn) {
      const landsInHand = state.hand.filter(c => c.type === 'land') as LandCard[];
      landsInHand.sort((a, b) => {
        const aUntapped = willLandEnterUntapped(a, state);
        const bUntapped = willLandEnterUntapped(b, state);
        if (aUntapped && !bUntapped) return -1;
        if (bUntapped && !aUntapped) return 1;
        return 0;
      });

      if (landsInHand.length > 0) {
        const land = landsInHand[0];
        state.hand = state.hand.filter(c => c !== land);
        const tapped = !willLandEnterUntapped(land, state);
        // Pay life for shock lands
        if (land.entersTapped === 'conditional' && land.subtype !== 'fastland' && land.subtype !== 'town' && state.life > 2) {
          state.life -= 2;
        }
        state.battlefield.push({ card: land, tapped, turnEntered: t });
        state.landPlayedThisTurn = true;
      }
    }
  }

  // After turn 4, untap everything for the check
  for (const p of state.battlefield) {
    p.tapped = false;
  }

  // Check turn 4 state
  const lands = state.battlefield.filter(p => p.card.type === 'land');
  const untappedLands = lands.filter(p => !p.tapped);
  const hasSpiderMan = state.hand.some(c => c.name === 'Superior Spider-Man');
  const hasBringerInGY = state.graveyard.some(c => c.name === 'Bringer of the Last Gift');
  const hasTerrorInGY = state.graveyard.some(c => c.name === 'Terror of the Peaks');

  // Check if we could cast Spider-Man (need 4 mana with U and B)
  const canCastSpider = untappedLands.length >= 4 &&
    untappedLands.some(p => canTapForMana(p, state).includes('U')) &&
    untappedLands.some(p => canTapForMana(p, state).includes('B'));

  // What's missing?
  let reason = '';
  if (!hasSpiderMan) reason = 'No Spider-Man in hand';
  else if (lands.length < 4) reason = `Only ${lands.length} lands total`;
  else if (!canCastSpider) reason = 'Missing U or B colors';
  else if (!hasBringerInGY) reason = 'Bringer not in GY';
  else if (!hasTerrorInGY) reason = 'Terror not in GY (can still combo)';
  else {
    reason = 'TURN 4 WIN POSSIBLE!';
    turn4Possible++;
  }

  turn4Reasons[reason] = (turn4Reasons[reason] || 0) + 1;
}

console.log(`\nTurn 4 win possible in ${turn4Possible}/1000 games (${(turn4Possible / 10).toFixed(1)}%)`);
console.log(`\nReasons for no turn 4 win:`);
for (const [reason, count] of Object.entries(turn4Reasons).sort((a, b) => b[1] - a[1])) {
  console.log(`  ${reason}: ${count} (${(count / 10).toFixed(1)}%)`);
}

// NEW: Run actual game simulation and check turn 4 state
console.log(`\n--- Turn 4 State Check (1000 FULL game simulations) ---`);
let turn4Ready = 0;
let turn4ActualWins = 0;
const turn4Details: Record<string, number> = {};

for (let i = 0; i < 1000; i++) {
  // Run the actual game but stop and check at START of turn 4 (after untap/draw)
  const state = createInitialGameState();
  state.library = shuffle(buildDeck());
  selectOpeningHand(state);

  // Play turns 1-3 fully
  for (let t = 1; t <= 3; t++) {
    state.turn = t;
    state.landPlayedThisTurn = false;

    // Untap
    for (const p of state.battlefield) {
      p.tapped = false;
    }

    // Draw (skip turn 1 on play)
    if (t > 1 || !state.onThePlay) {
      drawCards(state, 1);
    }

    // Main phase
    mainPhase(state);
  }

  // Turn 4: just untap and draw, then check state BEFORE main phase
  state.turn = 4;
  state.landPlayedThisTurn = false;
  for (const p of state.battlefield) {
    p.tapped = false;
  }
  if (!state.onThePlay || state.turn > 1) {
    drawCards(state, 1);
  }

  // Now check turn 4 state BEFORE main phase (all lands untapped)
  const hasSpiderMan = state.hand.some(c => c.name === 'Superior Spider-Man');
  const hasBringerInGY = state.graveyard.some(c => c.name === 'Bringer of the Last Gift');
  const hasTerrorInGY = state.graveyard.some(c => c.name === 'Terror of the Peaks');
  const landsInHand = state.hand.filter(c => c.type === 'land').length;
  const landsOnField = state.battlefield.filter(p => p.card.type === 'land').length;

  // Check if we can cast Spider-Man (need 4 mana with U and B)
  // Count potential lands: 3 on field + 1 we can play = 4
  const potentialLands = landsOnField + (landsInHand > 0 ? 1 : 0);
  const hasU = state.battlefield.some(p => p.card.type === 'land' && canTapForMana(p, state).includes('U')) ||
               state.hand.some(c => c.type === 'land' && (c as LandCard).colors.includes('U'));
  const hasB = state.battlefield.some(p => p.card.type === 'land' && canTapForMana(p, state).includes('B')) ||
               state.hand.some(c => c.type === 'land' && (c as LandCard).colors.includes('B'));

  // Check if lands we'd have are untapped (need 4 untapped mana)
  // Lands from turns 1-3 are untapped, land we play turn 4 might be tapped
  const untappedLandsOnField = landsOnField; // All untapped after untap step

  // Check if we have an untapped land to play (shock lands count if we can pay life)
  const untappedLandInHand = state.hand.find(c => {
    if (c.type !== 'land') return false;
    const land = c as LandCard;
    return land.entersTapped === false || (land.entersTapped === 'conditional' && state.life > 2);
  });
  const landInHandUntapped = untappedLandInHand !== undefined;
  const totalUntappedMana = untappedLandsOnField + (landInHandUntapped ? 1 : 0);

  const canCastSpider = hasSpiderMan && potentialLands >= 4 && totalUntappedMana >= 4 && hasU && hasB;

  if (canCastSpider && hasBringerInGY) {
    turn4Ready++;
    if (hasTerrorInGY) {
      turn4Details['READY: Spider + Bringer + Terror (can win T4!)'] = (turn4Details['READY: Spider + Bringer + Terror (can win T4!)'] || 0) + 1;
    } else {
      turn4Details['READY: Spider + Bringer (no Terror, can still combo)'] = (turn4Details['READY: Spider + Bringer (no Terror, can still combo)'] || 0) + 1;
    }
  } else if (hasSpiderMan && hasBringerInGY) {
    // Have pieces but can't cast
    if (potentialLands < 4) {
      turn4Details[`Only ${landsOnField} lands on field`] = (turn4Details[`Only ${landsOnField} lands on field`] || 0) + 1;
    } else if (!hasU || !hasB) {
      turn4Details['Missing U or B color'] = (turn4Details['Missing U or B color'] || 0) + 1;
    } else if (totalUntappedMana < 4) {
      turn4Details['Not enough untapped mana (tapped lands)'] = (turn4Details['Not enough untapped mana (tapped lands)'] || 0) + 1;
    } else {
      turn4Details['Unknown mana issue'] = (turn4Details['Unknown mana issue'] || 0) + 1;
    }
  } else if (hasSpiderMan && !hasBringerInGY) {
    turn4Details['Spider-Man in hand, Bringer not milled'] = (turn4Details['Spider-Man in hand, Bringer not milled'] || 0) + 1;
  } else if (!hasSpiderMan && hasBringerInGY) {
    turn4Details['Bringer milled, no Spider-Man'] = (turn4Details['Bringer milled, no Spider-Man'] || 0) + 1;
  } else {
    turn4Details['Neither piece ready'] = (turn4Details['Neither piece ready'] || 0) + 1;
  }
}

console.log(`\nTurn 4 combo ready (can win): ${turn4Ready}/1000 (${(turn4Ready / 10).toFixed(1)}%)`);
console.log(`\nBreakdown:`);
for (const [detail, count] of Object.entries(turn4Details).sort((a, b) => b[1] - a[1])) {
  console.log(`  ${detail}: ${count} (${(count / 10).toFixed(1)}%)`);
}

// Now let's see if the game actually wins on turn 4 when ready
console.log(`\n--- Verifying Turn 4 Wins ---`);
let actualT4Wins = 0;

for (let i = 0; i < 1000; i++) {
  const result = runGame();
  if (result.winTurn === 4) {
    actualT4Wins++;
  }
}
console.log(`Actual Turn 4 wins in 1000 games: ${actualT4Wins} (${(actualT4Wins / 10).toFixed(1)}%)`);
console.log(`\nCompare: ${turn4Ready} games were ready at start of T4, but only ${actualT4Wins} won on T4.`);
console.log(`Gap suggests ${turn4Ready - actualT4Wins} games had pieces but didn't execute the combo.`);

// Debug: Run a game with T4 ready state and trace what happens
console.log(`\n--- Debug: Why T4 combo doesn't fire ---`);
let debugCount = 0;
let trueReadyCount = 0;
for (let i = 0; i < 10000 && debugCount < 5; i++) {
  const state = createInitialGameState();
  state.library = shuffle(buildDeck());
  selectOpeningHand(state);

  // Play turns 1-3
  for (let t = 1; t <= 3; t++) {
    state.turn = t;
    state.landPlayedThisTurn = false;
    for (const p of state.battlefield) p.tapped = false;
    if (t > 1 || !state.onThePlay) drawCards(state, 1);
    mainPhase(state);
  }

  // Turn 4: untap and draw
  state.turn = 4;
  state.landPlayedThisTurn = false;
  for (const p of state.battlefield) p.tapped = false;
  drawCards(state, 1);

  // Check if ready
  const hasSpiderMan = state.hand.some(c => c.name === 'Superior Spider-Man');
  const hasBringerInGY = state.graveyard.some(c => c.name === 'Bringer of the Last Gift');

  if (hasSpiderMan && hasBringerInGY) {
    // Check if TRULY ready: 3 lands on field + untapped land in hand + has U and B
    const landsOnField = state.battlefield.filter(p => p.card.type === 'land').length;
    const hasU = state.battlefield.some(p => p.card.type === 'land' && canTapForMana(p, state).includes('U'));
    const hasB = state.battlefield.some(p => p.card.type === 'land' && canTapForMana(p, state).includes('B'));
    const untappedLandInHand = state.hand.find(c => {
      if (c.type !== 'land') return false;
      const land = c as LandCard;
      return land.entersTapped === false || (land.entersTapped === 'conditional' && state.life > 2);
    });

    const trulyReady = landsOnField >= 3 && hasU && hasB && untappedLandInHand;

    if (trulyReady) {
      trueReadyCount++;
      debugCount++;
      console.log(`\n--- Debug game ${debugCount} (TRULY READY) ---`);
      console.log(`Turn 4 start: Spider-Man in hand, Bringer in GY`);
      console.log(`Lands on field (${landsOnField}): ${state.battlefield.filter(p => p.card.type === 'land').map(p => p.card.name).join(', ')}`);
      console.log(`Hand: ${state.hand.map(c => c.name).join(', ')}`);
      console.log(`Untapped land in hand: ${untappedLandInHand.name}`);
      console.log(`Colors available: U=${hasU}, B=${hasB}`);

      // Try to cast Spider-Man
      const spiderCard = state.hand.find(c => c.name === 'Superior Spider-Man')!;
      console.log(`Can cast Spider-Man (before land): ${canCastSpell(spiderCard, state)}`);
      console.log(`Max available mana (before land): ${getMaxAvailableMana(state)}`);

      // Now run main phase and see what happens
      const opponentLifeBefore = state.opponentLife;
      mainPhase(state);

      // Check if combo fired
      const bringerCopyOnField = state.battlefield.some(p => (p as any).isCopyOf === 'Bringer of the Last Gift');
      console.log(`After main phase - Combo fired: ${bringerCopyOnField}`);
      console.log(`Opponent life: ${state.opponentLife} (was ${opponentLifeBefore})`);
      console.log(`Lands on field after main phase: ${state.battlefield.filter(p => p.card.type === 'land').length}`);

      // Check if Spider-Man was cast
      const spiderOnField = state.battlefield.some(p => p.card.name === 'Superior Spider-Man');
      console.log(`Spider-Man on battlefield: ${spiderOnField}`);
    }
  }
}
console.log(`\nFound ${trueReadyCount} truly ready games in 10000 samples`);

// FINAL: Detailed turn 4 combo analysis - count exact scenarios
console.log(`\n=== FINAL Turn 4 Analysis: Why Not Lethal? ===`);
let t4ComboCounts = {
  comboFired: 0,
  comboFiredWithTerror: 0,
  comboFiredNoTerror: 0,
  lethalDamage: 0,
  notLethalDamage: 0,
  damageDealt: [] as number[],
  creaturesReanimated: [] as number[],
  terrorsOnCombo: [] as number[],
};

for (let i = 0; i < 10000; i++) {
  const state = initializeGame();

  // Play through turn 4 and track if combo fires
  for (let t = 1; t <= 4; t++) {
    if (state.opponentLife <= 0) break;
    playTurn(state);
  }

  // Check if we won by turn 4 (combo fired and was lethal)
  if (state.turn === 4 || (state.turn <= 4 && state.opponentLife <= 0)) {
    // Check if Spider-Man copied Bringer
    const spiderCopiedBringer = state.battlefield.some(p => p.isCopyOf === 'Bringer of the Last Gift');

    if (spiderCopiedBringer) {
      t4ComboCounts.comboFired++;

      // Count Terrors on battlefield when combo fired
      const terrorCount = state.battlefield.filter(p => p.card.name === 'Terror of the Peaks').length;
      t4ComboCounts.terrorsOnCombo.push(terrorCount);

      if (terrorCount > 0) {
        t4ComboCounts.comboFiredWithTerror++;
      } else {
        t4ComboCounts.comboFiredNoTerror++;
      }

      // Count creatures that were reanimated (all creatures except Spider-Man itself)
      const creaturesOnField = state.battlefield.filter(p => p.card.type === 'creature').length;
      t4ComboCounts.creaturesReanimated.push(creaturesOnField);

      // Calculate damage dealt (opponent started at 20)
      const damageDealt = 20 - state.opponentLife;
      t4ComboCounts.damageDealt.push(damageDealt);

      if (state.opponentLife <= 0) {
        t4ComboCounts.lethalDamage++;
      } else {
        t4ComboCounts.notLethalDamage++;
      }
    }
  }
}

console.log(`\nCombo fired by turn 4: ${t4ComboCounts.comboFired}/10000 (${(t4ComboCounts.comboFired / 100).toFixed(2)}%)`);
console.log(`  - With Terror: ${t4ComboCounts.comboFiredWithTerror}`);
console.log(`  - Without Terror: ${t4ComboCounts.comboFiredNoTerror}`);
console.log(`  - Lethal: ${t4ComboCounts.lethalDamage}`);
console.log(`  - Not lethal: ${t4ComboCounts.notLethalDamage}`);

if (t4ComboCounts.damageDealt.length > 0) {
  const avgDamage = t4ComboCounts.damageDealt.reduce((a, b) => a + b, 0) / t4ComboCounts.damageDealt.length;
  const minDamage = Math.min(...t4ComboCounts.damageDealt);
  const maxDamage = Math.max(...t4ComboCounts.damageDealt);
  const avgCreatures = t4ComboCounts.creaturesReanimated.reduce((a, b) => a + b, 0) / t4ComboCounts.creaturesReanimated.length;
  const avgTerrors = t4ComboCounts.terrorsOnCombo.reduce((a, b) => a + b, 0) / t4ComboCounts.terrorsOnCombo.length;

  console.log(`\nWhen combo fires by T4:`);
  console.log(`  Average damage dealt: ${avgDamage.toFixed(1)}`);
  console.log(`  Damage range: ${minDamage} - ${maxDamage}`);
  console.log(`  Average creatures reanimated: ${avgCreatures.toFixed(1)}`);
  console.log(`  Average Terrors: ${avgTerrors.toFixed(2)}`);

  // Histogram of damage
  console.log(`\nDamage distribution when combo fires:`);
  const damageHist: Record<number, number> = {};
  for (const d of t4ComboCounts.damageDealt) {
    const bucket = Math.floor(d / 5) * 5; // Group by 5s
    damageHist[bucket] = (damageHist[bucket] || 0) + 1;
  }
  for (const [bucket, count] of Object.entries(damageHist).sort((a, b) => Number(a[0]) - Number(b[0]))) {
    const bar = "█".repeat(Math.ceil(count / 2));
    console.log(`  ${bucket}-${Number(bucket) + 4}: ${bar} (${count})`);
  }
}

} // End of else block (normal mode)

