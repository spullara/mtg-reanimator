# Reanimator Deck Playstyle Guide

This document describes the strategic use of each card in the Sultai Reanimator deck, based on the simulator's decision engine.

## The Combo

**Win Condition:** Cast **Superior Spider-Man** when **Bringer of the Last Gift** is in the graveyard.

1. Spider-Man's "mind swap copy" ability copies Bringer as it enters the battlefield
2. The copied Bringer triggers, sacrificing all creatures and reanimating everything from the graveyard
3. **Terror of the Peaks** deals damage equal to each creature's power as they enter
4. With 2+ Terrors and multiple large creatures, this easily deals 20+ damage

**Timing:** The simulator holds Spider-Man until the combo is **lethal** (expected damage ≥ opponent's life).

---

## Creatures

### Superior Spider-Man (4 mana - 1U1B2)
**Role:** Combo finisher - copies Bringer to trigger mass reanimation

**When to cast:**
- ✅ Bringer is in the graveyard AND combo damage would be lethal
- ❌ Never cast "just as a creature" - always wait for lethal combo

**Mill priority:** Always return to hand when milled (highest priority)

---

### Bringer of the Last Gift (8 mana - 2B6)
**Role:** Reanimation engine - never cast, always reanimate

**Strategy:**
- ✅ Discard via Kiora's ETB ability
- ✅ Mill directly into graveyard
- ❌ Never return from graveyard to hand - it belongs there!
- ❌ Never hardcast (8 mana is too slow)

**Creature type:** Demon (relevant for Cavern of Souls)

---

### Terror of the Peaks (5 mana - 2R3)
**Role:** Damage multiplier - deals power damage for each creature entering

**Strategy:**
- ✅ Discard via Kiora (same as Bringer)
- ✅ Mill into graveyard for reanimation
- ❌ Never return from graveyard - keep it there for the combo

**The more Terrors in graveyard, the more damage the combo deals!**

---

### Kiora, the Rising Tide (3 mana - 1U2)
**Role:** Card filtering + discard outlet for combo pieces

**ETB Ability:** Draw 2, discard 2

**Priority:** Cast immediately if Bringer or Terror is stuck in hand

**When to cast:**
- ✅ Bringer/Terror in hand → cast ASAP to discard them
- ✅ Need card filtering
- ⚠️ Lower priority than mill spells if no combo pieces in hand

**Creature type:** Noble (for second Cavern of Souls)

---

### Town Greeter (2 mana - 1G1)
**Role:** Early mill + mana fixing

**ETB Ability:** Mill 4, return a land to hand

**When to cast:**
- ✅ Early game (< 4 lands) for mana development
- ✅ Need to find blue-producing lands
- ⚠️ Lower priority than Dredger's Insight (which can return creatures)

**Creature type:** Human (uses first Cavern)

---

### Overlord of the Balemurk (5 mana or 2 impending - 2B3 / 1B1)
**Role:** Repeatable mill engine

**Abilities:**
- **Impending 5:** Cast for 1B1, enters with 5 time counters (no ETB)
- **ETB/Attack:** Mill 4, return a creature or planeswalker to hand

**Overlord Return Priority:**
1. Spider-Man if Bringer is in graveyard
2. Kiora if Bringer is stuck in hand  
3. Town Greeter if early game (< 4 lands)
4. Otherwise: return nothing (keep creatures in graveyard for reanimation!)

---

## Spells

### Cache Grab (2 mana - 1G1) - Instant
**Role:** Primary mill spell

**Effect:** Mill 4, return any permanent to hand

**Cast priority:** High - cast before other spells when mana allows

**Mill return priority:**
1. Superior Spider-Man
2. Kiora
3. Blue-producing lands (Watery Grave, Undercity Sewers)
4. Other dual lands
5. Basic lands
6. Non-combo creatures
7. ❌ Never return Bringer or Terror

---

### Dredger's Insight (2 mana - 1G1) - Enchantment
**Role:** Mill + value engine

**ETB Effect:** Mill 4, return artifact/creature/land to hand

**Cast priority:** High - cheap enabler that stays on battlefield

**Mill return priority:** Same as Cache Grab - prioritize Spider-Man, Kiora, then blue lands

---

### Awaken the Honored Dead (3 mana - 1U1B1G) - Saga
**Role:** Multi-purpose utility

**Chapters:**
1. Destroy target nonland permanent (removal)
2. Mill 3 cards
3. Discard a card, return creature or land from graveyard

**When to cast:** Lower priority than Cache Grab/Dredger's - costs 3 colors

---

## Lands

### Land Play Priority

When choosing which land to play:

1. **Enable casting this turn** - untapped land that provides missing colors for a spell in hand
2. **Provide missing colors** - fill in U/B/G even if can't cast anything yet
3. **Surveil lands** - get extra value (Undercity Sewers, Underground Mortuary)
4. **Tapped lands** - save untapped lands for later when colors don't matter

### Cavern of Souls
**Creature Type Selection:**

| Situation | Choose | Reason |
|-----------|--------|--------|
| First Cavern | **Human** | Helps Spider-Man and Town Greeter |
| Kiora + Bringer/Terror in hand, another Cavern coming | **Noble** | Cast Kiora first to discard |
| Already have Human Cavern | Based on hand | Demon (Bringer), Noble (Kiora), Dragon (Terror), Avatar (Overlord) |

### Multiversal Passage
**Color Selection:** Checks what colors are missing for spells in hand.

Priority: G → U → B (default to U for Spider-Man/Kiora if no specific need)

### Fastlands (Blooming Marsh)
Enter untapped if you control ≤ 2 lands. Save for early game if possible.

### Town Lands (Starting Town)
Enter untapped until turn 3, then tapped. Play early!

### Surveil Lands (Undercity Sewers, Underground Mortuary)
Always enter tapped, but surveil 1 provides mill value. Good for turns when you can't cast anything anyway.

---

## Mulligan Strategy

**Keep if:**
- 2+ lands AND
- At least one mill enabler OR playable early spell

**Mill Enablers:**
- Dredger's Insight
- Cache Grab
- Town Greeter
- Overlord of the Balemurk
- Kiora, the Rising Tide
- Awaken the Honored Dead

**Always mulligan if:**
- 0-1 lands (can't develop mana)
- 6+ lands (no action)
- No way to start milling

**BO1 Hand Smoother:**
Draws 2 hands of 7, keeps the one with better land count (2-5 lands preferred).

---

## Discard Priority (Kiora ETB)

When Kiora's "draw 2, discard 2" triggers:

1. ✅ **Bringer of the Last Gift** - wants to be in graveyard
2. ✅ **Terror of the Peaks** - wants to be in graveyard
3. ✅ Excess lands
4. ✅ Expensive spells (mana value 4+)
5. ❌ Never discard the last Spider-Man (need to cast it!)

---

## Turn-by-Turn Example

**Ideal game:**

| Turn | Play | Result |
|------|------|--------|
| 1 | Land | Mana: G |
| 2 | Land + Dredger's Insight | Mill 4 → return blue land |
| 3 | Blue land + Kiora | Draw 2, discard Bringer + Terror |
| 4 | Land → Spider-Man | COMBO! Mill 4, Spider-Man copies Bringer |

**Average win turn:** ~6.5 turns

---

## Key Strategic Principles

1. **Never hardcast Bringer or Terror** - they're too expensive and better reanimated
2. **Never return Bringer/Terror from graveyard** - they need to stay there
3. **Always return Spider-Man from mill** - it's the combo piece you cast
4. **Prioritize blue mana** - Kiora (2U) and Spider-Man (1U1B) both need blue
5. **Hold Spider-Man until lethal** - don't waste the combo on non-lethal damage
6. **Mill aggressively early** - get combo pieces in graveyard ASAP
7. **Kiora is key** - the best way to get Bringer/Terror from hand to graveyard

