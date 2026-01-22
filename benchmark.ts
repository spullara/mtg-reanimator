/**
 * MTG Simulator Benchmark - Bun/TypeScript
 * Runs 500k games and measures throughput
 */

import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

// ============================================================================
// MINIMAL PRNG (Mulberry32)
// ============================================================================

let currentSeed = 42;

function mulberry32(seed: number): () => number {
  return function() {
    let t = seed += 0x6D2B79F5;
    t = Math.imul(t ^ t >>> 15, t | 1);
    t ^= t + Math.imul(t ^ t >>> 7, t | 61);
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  };
}

let random = mulberry32(currentSeed);

function setSeed(seed: number): void {
  currentSeed = seed;
  random = mulberry32(seed);
}

// ============================================================================
// TYPES
// ============================================================================

type ManaColor = "W" | "U" | "B" | "R" | "G" | "C";

interface ManaCost {
  W?: number; U?: number; B?: number; R?: number; G?: number; C?: number;
  generic?: number;
}

interface ManaPool {
  W: number; U: number; B: number; R: number; G: number; C: number;
}

type CardType = "land" | "creature" | "instant" | "sorcery" | "enchantment" | "saga";

interface BaseCard {
  name: string;
  type: CardType;
  manaCost?: ManaCost;
  manaValue: number;
}

interface LandCard extends BaseCard {
  type: "land";
  subtype: string;
  entersTapped: boolean | "conditional";
  colors: ManaColor[];
  hasSurveil?: boolean;
  surveilAmount?: number;
}

interface CreatureCard extends BaseCard {
  type: "creature";
  power: number;
  toughness: number;
  creatureTypes: string[];
  abilities: string[];
  impendingCost?: ManaCost;
  impendingCounters?: number;
}

type Card = LandCard | CreatureCard | BaseCard;

interface BattlefieldPermanent {
  card: Card;
  tapped: boolean;
  turnEntered: number;
  counters?: { time?: number };
  isCopyOf?: string;
  chosenType?: string;
  chosenBasicType?: ManaColor;
}

interface GameState {
  library: Card[];
  hand: Card[];
  graveyard: Card[];
  battlefield: BattlefieldPermanent[];
  exile: Card[];
  turn: number;
  phase: string;
  onThePlay: boolean;
  landPlayedThisTurn: boolean;
  life: number;
  opponentLife: number;
  manaPool: ManaPool;
}

// ============================================================================
// CARD DATABASE (SIMPLIFIED)
// ============================================================================

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
  impending_cost?: Record<string, number>;
  impending_counters?: number;
}

function convertColorName(color: string): ManaColor {
  const colorMap: Record<string, ManaColor> = {
    white: "W", blue: "U", black: "B", red: "R", green: "G", colorless: "C",
    W: "W", U: "U", B: "B", R: "R", G: "G", C: "C",
  };
  return colorMap[color] || (color as ManaColor);
}

function convertManaCost(jsonCost: Record<string, number> | undefined): ManaCost | undefined {
  if (!jsonCost) return undefined;
  const result: ManaCost = {};
  for (const [key, value] of Object.entries(jsonCost)) {
    if (key === "generic") result.generic = value;
    else result[convertColorName(key)] = value;
  }
  return result;
}

function loadCardDatabase(): Record<string, Card> {
  const __filename = fileURLToPath(import.meta.url);
  const __dirname = dirname(__filename);
  const cardsPath = join(__dirname, "cards.json");
  const jsonCards: JsonCard[] = JSON.parse(readFileSync(cardsPath, "utf-8"));
  
  const database: Record<string, Card> = {};
  for (const json of jsonCards) {
    let card: Card;
    if (json.card_type === "land") {
      card = {
        name: json.name,
        type: "land",
        manaValue: json.mana_value,
        subtype: json.subtype || "basic",
        entersTapped: json.subtype === "shock" || json.subtype === "fastland" || json.subtype === "town" 
          ? "conditional" : (json.enters_tapped ?? false),
        colors: (json.colors || []).map(convertColorName),
        hasSurveil: json.has_surveil,
        surveilAmount: json.surveil_amount,
      } as LandCard;
    } else if (json.card_type === "creature") {
      card = {
        name: json.name,
        type: "creature",
        manaValue: json.mana_value,
        manaCost: convertManaCost(json.mana_cost),
        power: json.power || 0,
        toughness: json.toughness || 0,
        creatureTypes: json.creature_types || [],
        abilities: json.abilities || [],
        impendingCost: convertManaCost(json.impending_cost),
        impendingCounters: json.impending_counters,
      } as CreatureCard;
    } else {
      card = {
        name: json.name,
        type: json.card_type as CardType,
        manaValue: json.mana_value,
        manaCost: convertManaCost(json.mana_cost),
      };
    }
    database[json.name] = card;
  }
  return database;
}

const CARD_DATABASE = loadCardDatabase();

// ============================================================================
// DECK LOADING
// ============================================================================

function buildDeck(filename = "deck.txt"): Card[] {
  const content = readFileSync(filename, "utf-8");
  const deck: Card[] = [];
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;
    const match = trimmed.match(/^(\d+)\s+(.+)$/);
    if (match) {
      const count = parseInt(match[1], 10);
      const cardName = match[2].trim();
      const card = CARD_DATABASE[cardName];
      if (card) {
        for (let i = 0; i < count; i++) deck.push({ ...card });
      }
    }
  }
  return deck;
}

// ============================================================================
// GAME LOGIC (SIMPLIFIED FOR BENCHMARK)
// ============================================================================

function shuffle<T>(array: T[]): T[] {
  const result = [...array];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

function emptyManaPool(): ManaPool {
  return { W: 0, U: 0, B: 0, R: 0, G: 0, C: 0 };
}

function createGameState(): GameState {
  return {
    library: [],
    hand: [],
    graveyard: [],
    battlefield: [],
    exile: [],
    turn: 0,
    phase: "main",
    onThePlay: random() < 0.5,
    landPlayedThisTurn: false,
    life: 20,
    opponentLife: 20,
    manaPool: emptyManaPool(),
  };
}

function countLands(cards: Card[]): number {
  let count = 0;
  for (const c of cards) if (c.type === "land") count++;
  return count;
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

function selectOpeningHand(state: GameState): void {
  const hand1 = state.library.splice(0, 7);
  const hand2 = state.library.splice(0, 7);
  const lands1 = countLands(hand1);
  const lands2 = countLands(hand2);
  
  let chosenHand: Card[], rejectedHand: Card[];
  if (lands1 >= 2 && lands2 >= 2) {
    if (lands1 <= lands2) { chosenHand = hand1; rejectedHand = hand2; }
    else { chosenHand = hand2; rejectedHand = hand1; }
  } else if (lands1 >= 2) { chosenHand = hand1; rejectedHand = hand2; }
  else if (lands2 >= 2) { chosenHand = hand2; rejectedHand = hand1; }
  else {
    state.library = shuffle([...state.library, ...hand1, ...hand2]);
    drawCards(state, 6);
    return;
  }
  state.library = shuffle([...state.library, ...rejectedHand]);
  state.hand = chosenHand;
}

function canTapForMana(permanent: BattlefieldPermanent, state: GameState): ManaColor[] {
  if (permanent.tapped || permanent.card.type !== "land") return [];
  const land = permanent.card as LandCard;
  if (land.name === "Cavern of Souls") return ["C"];
  if (land.name === "Starting Town") return state.life > 1 ? ["C", "W", "U", "B", "R", "G"] : ["C"];
  if (land.name === "Multiversal Passage" && permanent.chosenBasicType) return [permanent.chosenBasicType];
  return [...land.colors];
}

function getMaxMana(state: GameState): number {
  let count = 0;
  for (const p of state.battlefield) {
    if (p.card.type === "land" && !p.tapped) count++;
  }
  return count;
}

function playLand(state: GameState, card: Card): boolean {
  if (state.landPlayedThisTurn || card.type !== "land") return false;
  const idx = state.hand.indexOf(card);
  if (idx === -1) return false;
  
  state.hand.splice(idx, 1);
  const land = card as LandCard;
  let tapped = land.entersTapped === true;
  if (land.entersTapped === "conditional") {
    if (land.subtype === "fastland") tapped = state.battlefield.filter(p => p.card.type === "land").length > 2;
    else if (land.subtype === "town") tapped = state.turn > 3;
    else if (state.life > 2) { state.life -= 2; tapped = false; }
    else tapped = true;
  }
  
  const permanent: BattlefieldPermanent = { card, tapped, turnEntered: state.turn };
  if (land.name === "Cavern of Souls") permanent.chosenType = "Human";
  if (land.name === "Multiversal Passage") permanent.chosenBasicType = "B";
  state.battlefield.push(permanent);
  state.landPlayedThisTurn = true;
  
  if (land.hasSurveil) {
    for (let i = 0; i < (land.surveilAmount || 1) && state.library.length > 0; i++) {
      const top = state.library[0];
      if (top.name === "Bringer of the Last Gift" || top.name === "Terror of the Peaks") {
        state.library.shift();
        state.graveyard.push(top);
      }
    }
  }
  return true;
}

function tapLandsForMana(cost: ManaCost, state: GameState): boolean {
  const total = (cost.W || 0) + (cost.U || 0) + (cost.B || 0) + (cost.R || 0) + (cost.G || 0) + (cost.generic || 0);
  if (getMaxMana(state) < total) return false;
  
  let needed = total;
  for (const p of state.battlefield) {
    if (needed <= 0) break;
    if (p.card.type === "land" && !p.tapped) {
      p.tapped = true;
      needed--;
    }
  }
  return needed <= 0;
}

function castSpell(state: GameState, card: Card): boolean {
  if (!card.manaCost) return false;
  const idx = state.hand.indexOf(card);
  if (idx === -1) return false;
  
  const creature = card.type === "creature" ? card as CreatureCard : undefined;
  const cost = creature?.impendingCost || card.manaCost;
  if (!tapLandsForMana(cost, state)) return false;
  
  state.hand.splice(idx, 1);
  
  if (card.type === "creature") {
    const perm: BattlefieldPermanent = { card, tapped: false, turnEntered: state.turn };
    if (creature?.impendingCost && creature.impendingCounters) {
      perm.counters = { time: creature.impendingCounters };
    }
    state.battlefield.push(perm);
    
    // Simplified ETB effects
    if (card.name === "Kiora, the Rising Tide") {
      drawCards(state, 2);
      for (let i = 0; i < 2 && state.hand.length > 0; i++) {
        const discard = state.hand.find(c => c.name === "Bringer of the Last Gift") || 
                        state.hand.find(c => c.name === "Terror of the Peaks") ||
                        state.hand[state.hand.length - 1];
        const di = state.hand.indexOf(discard);
        if (di !== -1) { state.hand.splice(di, 1); state.graveyard.push(discard); }
      }
    } else if (card.name === "Town Greeter" || card.name === "Overlord of the Balemurk") {
      mill(state, 4);
    } else if (card.name === "Superior Spider-Man") {
      const bringer = state.graveyard.find(c => c.name === "Bringer of the Last Gift");
      if (bringer) {
        perm.isCopyOf = "Bringer of the Last Gift";
        const bi = state.graveyard.indexOf(bringer);
        state.graveyard.splice(bi, 1);
        state.exile.push(bringer);
        
        // Bringer ETB - reanimate all
        const creatures = state.graveyard.filter(c => c.type === "creature");
        state.graveyard = state.graveyard.filter(c => c.type !== "creature");
        
        let terrorCount = state.battlefield.filter(p => 
          p.card.name === "Terror of the Peaks" || p.isCopyOf === "Terror of the Peaks"
        ).length;
        
        for (const c of creatures) {
          state.battlefield.push({ card: c, tapped: false, turnEntered: state.turn });
          if (c.name === "Terror of the Peaks") terrorCount++;
        }
        
        // Terror damage
        for (const c of creatures) {
          if (c.name !== "Terror of the Peaks") {
            state.opponentLife -= (c as CreatureCard).power * terrorCount;
          }
        }
      }
    }
  } else {
    // Spells
    if (card.name === "Cache Grab" || card.name === "Dredger's Insight") {
      mill(state, 4);
    }
    state.graveyard.push(card);
  }
  return true;
}

function mainPhase(state: GameState): void {
  // Play land
  const land = state.hand.find(c => c.type === "land");
  if (land) playLand(state, land);
  
  // Priority: Spider-Man if Bringer in GY
  const hasBringer = state.graveyard.some(c => c.name === "Bringer of the Last Gift");
  if (hasBringer) {
    const spider = state.hand.find(c => c.name === "Superior Spider-Man");
    if (spider && getMaxMana(state) >= 4) castSpell(state, spider);
  }
  
  // Cast other spells
  for (const card of [...state.hand]) {
    if (card.type !== "land" && card.manaCost) {
      castSpell(state, card);
    }
  }
}

function combatPhase(state: GameState): void {
  for (const p of state.battlefield) {
    if (p.card.type === "creature" && !p.tapped && p.turnEntered < state.turn) {
      if (!p.counters?.time) {
        const power = p.isCopyOf ? (CARD_DATABASE[p.isCopyOf] as CreatureCard).power : (p.card as CreatureCard).power;
        state.opponentLife -= power;
        p.tapped = true;
      }
    }
  }
}

function playTurn(state: GameState): void {
  state.turn++;
  state.landPlayedThisTurn = false;
  for (const p of state.battlefield) p.tapped = false;
  
  if (state.turn > 1 || !state.onThePlay) drawCards(state, 1);
  
  // End step: remove time counters
  for (const p of state.battlefield) {
    if (p.counters?.time && p.counters.time > 0) p.counters.time--;
  }
  
  mainPhase(state);
  combatPhase(state);
}

function runGame(): number {
  const state = createGameState();
  state.library = shuffle(buildDeck());
  selectOpeningHand(state);
  
  for (let turn = 1; turn <= 20; turn++) {
    playTurn(state);
    if (state.opponentLife <= 0) return state.turn;
  }
  return -1;
}

// ============================================================================
// BENCHMARK
// ============================================================================

const NUM_GAMES = 500000;
const SEED = 42;

console.log(`=== Bun/TypeScript MTG Simulator Benchmark ===`);
console.log(`Games: ${NUM_GAMES.toLocaleString()}`);
console.log(`Seed: ${SEED}`);
console.log();

setSeed(SEED);

const startTime = performance.now();

let wins = 0;
for (let i = 0; i < NUM_GAMES; i++) {
  const result = runGame();
  if (result > 0) wins++;
}

const endTime = performance.now();
const elapsed = (endTime - startTime) / 1000;
const gamesPerSec = Math.round(NUM_GAMES / elapsed);

console.log(`Completed in ${elapsed.toFixed(2)}s`);
console.log(`Throughput: ${gamesPerSec.toLocaleString()} games/sec`);
console.log(`Win rate: ${((wins / NUM_GAMES) * 100).toFixed(1)}%`);
