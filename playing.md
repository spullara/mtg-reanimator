# MTG Reanimator Simulator: How the AI Plays

This document explains how the simulator makes decisions when piloting the Sultai (Blue/Black/Green) Reanimator combo deck.

## Overall Strategy

### Win Condition
The deck wins by reanimating a massive board of creatures that deal lethal damage through **Terror of the Peaks** triggers. The core combo is:

1. Get **Bringer of the Last Gift** into the graveyard (via milling or discarding)
2. Get **Terror of the Peaks** into the graveyard (same way)
3. Cast **Superior Spider-Man**, copying Bringer of the Last Gift from the graveyard
4. When Spider-Man enters as a Bringer copy, it triggers mass reanimation—all creatures in the graveyard return to the battlefield
5. Each creature entering triggers Terror of the Peaks, dealing damage equal to that creature's power

A single Terror with a full graveyard can deal 20+ damage in one turn.

### The Game Plan
- **Turns 1-3**: Play lands, cast mill spells (Cache Grab, Dredger's Insight, Town Greeter, Overlord of the Balemurk) to fill the graveyard with combo pieces
- **Turn 4+**: Once Bringer is in graveyard and enough creatures are there for lethal Terror triggers, cast Superior Spider-Man to win

---

## Turn-by-Turn Decision Making

### Opening Hand Selection (BO1 Hand Smoother)

The simulator uses Arena's Best-of-One hand smoother:
1. Draw two hands of 7 cards
2. Pick the hand with **at least 2 lands** but **fewer total lands** (preferring action-heavy hands)
3. If both hands have 0-1 lands, mulligan to 6

### Mulligan Logic

If mulliganing:
- Keep any hand with 2+ lands down to 5 cards
- Below 5 cards, keep any hand
- After mulliganing, scry cards equal to cards below 7
- **Scry priority**: Put Bringer and Terror on the BOTTOM (we want them in the graveyard, not our hand!)

### Land Drop Priority

The simulator chooses which land to play based on this priority:

1. **Lands that enable casting a spell THIS turn** (untapped + provides needed colors)
2. **Lands that provide a missing color** (for future turns)
3. **Surveil lands** (Undercity Sewers, Underground Mortuary) for value—they can put combo pieces directly into the graveyard
4. **Tapped lands early** (save untapped lands for later when mana efficiency matters more)

Special land considerations:
- **Shock lands / Multiversal Passage**: Pay 2 life to enter untapped (if life > 2)
- **Blooming Marsh**: Enters untapped only if you control ≤2 other lands
- **Starting Town**: Enters untapped on turns 1-3 only

### Spell Casting Priority

Each turn, the simulator casts spells in this order:

1. **Superior Spider-Man** (only if combo would be LETHAL this turn—see below)
2. **Kiora, the Rising Tide** (if Bringer or Terror is stuck in hand—Kiora discards them!)
3. **Mill spells** to find combo pieces:
   - Cache Grab (instant, mill 4)
   - Dredger's Insight (enchantment, mill 4)
   - Town Greeter (creature, mill 4)
   - Overlord of the Balemurk (impending for 2 mana, mill 4)
4. **Awaken the Honored Dead** (saga that mills on chapter 2)
5. **Other spells** by mana cost (cheaper first to cast more per turn)

### Special Turn 4 Combo Check

On turn 4, if:
- Spider-Man is in hand
- Bringer is in graveyard
- You have 3 lands and an untapped land in hand

The simulator plays the land FIRST to enable the turn 4 combo, before casting any other spells.

---

## Key Decision Points

### When to Execute the Combo

The simulator **waits** to cast Spider-Man until the combo would be LETHAL. It calculates:

**Terror Damage** = (Power of each creature entering) × (Number of Terrors on battlefield or entering)

**Combat Damage** = Power of creatures that can attack this turn (no summoning sickness)

The combo only fires when: `Terror Damage + Combat Damage ≥ Opponent's Life`

This prevents wasting the combo when it wouldn't kill.

### How Mill/Surveil Targets Are Chosen

**What to PUT IN the graveyard** (surveil):
- Bringer of the Last Gift ✓
- Terror of the Peaks ✓
- Overlord of the Balemurk ✓
- Town Greeter (cheap, better to reanimate) ✓
- Extra copies of Kiora (if one already in hand) ✓

**What to KEEP on top**:
- Superior Spider-Man (CRITICAL—must stay in hand to cast!)
- Lands
- Other mill spells

### What to Return from Mill Effects

When Cache Grab, Dredger's Insight, or Overlord let you return a card to hand:

1. **Superior Spider-Man** — ALWAYS grab it (the key combo piece)
2. **Kiora** — if Bringer is stuck in hand (need to discard it)
3. **Lands** — only if desperate (0-1 lands on battlefield, none in hand)
4. **Mill enablers** (Town Greeter, Overlord, Kiora)

**NEVER return**: Bringer or Terror (they need to stay in the graveyard!)

### Kiora's Discard Priority

When Kiora enters (draw 2, discard 2):
1. Discard **Bringer of the Last Gift** first
2. Then discard **Terror of the Peaks**
3. Then excess lands (if holding >2)
4. Then anything else

### Cavern of Souls Creature Type

When playing Cavern of Souls:
- Default: **Human** (helps cast Spider-Man and Town Greeter)
- If already have a Human Cavern: **Demon** (for Bringer), **Noble** (for Kiora), or **Dragon** (for Terror)
- Special case: If Kiora + Bringer both in hand, set to **Noble** first (cast Kiora to discard Bringer)

### Multiversal Passage Color Choice

Choose the basic land type based on what colors are missing for spells in hand:
1. Fill missing Green (for Cache Grab, Town Greeter)
2. Fill missing Blue (for Spider-Man, Kiora)
3. Fill missing Black (for Spider-Man, Overlord)
4. Default to Blue

---

## Combo Execution: Step by Step

Here's exactly what happens when Superior Spider-Man copies Bringer:

1. **Cast Superior Spider-Man** (costs {2}{U}{B} = 4 mana)
2. Spider-Man's "Mind Swap" ability triggers—choose to copy Bringer of the Last Gift in graveyard
3. **Exile** the original Bringer from graveyard
4. Spider-Man enters as a 4/4 copy of Bringer (still named Superior Spider-Man)
5. **Bringer's ETB triggers** (because Spider-Man is now a copy):
   - All players sacrifice all OTHER creatures they control
   - Your sacrificed creatures go to graveyard
   - Then, ALL creature cards from ALL graveyards return to battlefield

6. **Terror of the Peaks triggers** for each creature entering:
   - Terror deals damage equal to each creature's power
   - If multiple Terrors enter, EACH one triggers for EACH creature
   - Terrors trigger for each other (but not themselves)

### Damage Calculation Example

Graveyard contains: Terror of the Peaks (5 power), Overlord (5), Kiora (3), Town Greeter (1)

When Spider-Man (entering as Bringer, 4/4) triggers the mass reanimate:
- Terror enters and triggers for: Overlord (5) + Kiora (3) + Town Greeter (1) = **9 damage**
- (Terror doesn't trigger for itself, and Spider-Man already entered before the mass reanimate)

With 2 Terrors in graveyard:
- Each Terror triggers for each other creature = 2 × (5+3+1) = 18 damage
- Plus each Terror triggers for the other Terror entering = 2 × 5 = 10 damage
- Total: **28 damage** (usually lethal!)

---

## Combat

The simulator attacks with ALL eligible creatures every turn:
- Must not have summoning sickness (entered on a previous turn)
- Must not be tapped
- Impending creatures (with time counters) cannot attack

Note: Reanimated creatures have summoning sickness and cannot attack the turn they enter.

---

## End of Turn

At end of turn:
1. Remove one time counter from impending permanents (like Overlord cast for impending)
2. Discard down to 7 cards if needed (prioritize discarding Bringer/Terror—they belong in the graveyard!)
3. Empty the mana pool

---

## Key Cards Explained

| Card | Role | Notes |
|------|------|-------|
| **Bringer of the Last Gift** | Win condition | 8 mana normally, but we never cast it—we cheat it into play via Spider-Man |
| **Terror of the Peaks** | Damage engine | Each creature entering deals damage equal to its power |
| **Superior Spider-Man** | Combo enabler | Copies Bringer from graveyard, triggering the mass reanimate |
| **Overlord of the Balemurk** | Mill engine | Cast for impending ({1}{B}) to mill 4 immediately; becomes a 5/5 after 5 turns |
| **Kiora, the Rising Tide** | Draw/discard | Gets Bringer/Terror out of hand into graveyard |
| **Cache Grab / Dredger's Insight** | Mill spells | Mill 4, return a permanent to hand (grab Spider-Man!) |
| **Town Greeter** | Mill + land | Mill 4, can return a land to hand |
| **Awaken the Honored Dead** | Value saga | Mills on chapter 2, returns creature on chapter 3 |
| **Analyze the Pollen** | Tutor | With evidence 8, searches for any creature (find Spider-Man) |

---

## Why the Simulator Waits to Combo

The simulator never "yolos" the combo early. If Spider-Man copying Bringer wouldn't deal lethal damage, the simulator:

1. Continues milling to put more creatures in the graveyard
2. Continues attacking with whatever creatures are on board
3. Waits until Terror triggers would be lethal

This is correct strategy—wasting the combo when it doesn't kill gives the opponent time to recover.

---

## Summary

The AI plays this deck by:
1. **Mulliganing** for hands with 2+ lands
2. **Playing tapped/surveil lands early** to set up mana and fill graveyard
3. **Casting mill spells** to find Bringer, Terror, and other creatures
4. **Using Kiora** to discard combo pieces stuck in hand
5. **Waiting** until Terror damage would be lethal
6. **Casting Spider-Man** to copy Bringer and win in one massive turn
