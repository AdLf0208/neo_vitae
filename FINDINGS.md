# NeoVitae — jar inventory and upstream findings (slice 1)

**Jar:** `ports/_shared/licence-cache/neovitae-1.21.1-1.1.51.jar` (12,348,069 B,
6,356 entries). `META-INF/neoforge.mods.toml`: `modId="neovitae"`,
`version="1.1.51"`, `license="MIT"`, displayName "Neo Vitae", authors WayofTime,
Saereth — **MIT, licence rank 0**. Kept as the licence evidence.

**Upstream:** <https://github.com/breakinblocks/NeoVitae>, clone at
`ports/_shared/upstream/neovitae`, commit `0abaa11f31add824a5d2be5018fbbe6a69b0a1bf`
(2026-11-08). Read for behaviour only; the port is a rewrite.

## What the jar actually contains (families, from `unzip -l`)

This is Blood Magic's successor, not a small altar mod. Registry/data families:

| Family | Jar evidence |
|---|---|
| Altar + multiblock | `data/neovitae/neovitae/altar_tier/{weak,apprentice,mage,master,archmage,transcendent}.json`; blocks `ara_vitae`, `incense_altar`, `blood_battery`, `blood_tank`, `orb_filling_link`, 244 blockstates |
| Runes | 21 blocks in `tags/block/altar/runes.json`: `rune_blank`, `rune_efficiency`, `rune_sacrifice`, `rune_sacrifice_self`, `rune_speed`, `rune_acceleration`, `rune_dislocation`, `rune_capacity`, `rune_capacity_augmented`, `rune_orb`, `rune_charging` and their reinforced `rune_2_*` twins |
| Orbs of Vitae | items `blood_orb_{weak,apprentice,magician,master,archmage,transcendent}`; data map `data/neovitae/data_maps/item/blood_orb_stats.json` |
| Altar recipes | 31 `neovitae:ara_vitae_recipe` files under `data/neovitae/recipe/ara_vitae/`, each with `bloodNeeded`, `craftSpeed`, `drainSpeed`, `minTier`, `input` |
| Other altars/machines | `athanor` (74 recipes), `alchemytable` (115), `hellfire_forge` (111), `flask` (113), `array` (48), `meteor`, `teleposer`, `path`, `dungeon` (197) |
| Rituals | `api/ritual/*`, `imperfect_ritual_stone`, `ritual_diviner*`, 197 dungeon recipes, structures/worldgen |
| Spiritus/Anima | `anima` network, incense/tranquility, sentient armour and tools (Scribe tools), sigils, anointments |
| Guides | 249 Modonomicon book entries, 566 advancements |

## The numbers this slice uses (each with its source)

| Fact | Value | Source |
|---|---|---|
| Altar main tank | `1000 (BUCKET_VOLUME) * 10 * capacityMod` mB | `common/blockentity/AraVitaeTile.java:870` |
| Altar I/O tank | `1000 * capacityMod` mB | `AraVitaeTile.java:874` |
| Reserve tank | `main * 20` mB | `AraVitaeTile.java:882`, `AltarConstants.RESERVE_CAPACITY_FACTOR` |
| Capacity rune | `capacity_mod` 0.2 (additive, one per **upgrade** position) | `data_maps/block/altar_rune_stats.json` |
| Augmented capacity rune | `augmented_capacity_power` 1.075 (multiplied) | same file |
| Base tick rate | 20 ticks, minimum 1 | `AltarConstants.BASE_TICK_RATE`, `MIN_TICK_RATE` |
| Structure check | every 100 ticks | `AltarConstants.STRUCTURE_CHECK_INTERVAL` |
| Crafting cooldown | 30 ticks | `AltarConstants.CRAFTING_COOLDOWN_TICKS` |
| Self-sacrifice conversion | 100 LP per health point | `ServerConfig.SELF_SACRIFICE_CONVERSION` default |
| Orb use sacrifice | 2 health (one heart) per use | `common/item/BloodOrbItem.java:use` |
| Orb altar range | 5 blocks | `BloodOrbItem.ORB_ALTAR_RANGE` |
| Weak altar layout | just the altar block, tier 0 | `data/neovitae/neovitae/altar_tier/weak.json` |
| Apprentice layout | altar + 8 runes at y-1; only the 4 orthogonal ones are `upgrade` and are scanned, so four Capacity Runes give 10,000 * (1 + 4*0.2) = 18,000 mB | `altar_tier/apprentice.json` |
| `world` orb recipe | 2,000 LP, `craftSpeed` 10, `drainSpeed` 0, input `#c:gems/diamond`, `minTier` 0 | `recipe/ara_vitae/weak_blood_orb.json` |
| `cobbled_deepslate` recipe | 50 LP, `craftSpeed` 5, `drainSpeed` 0, input `#c:cobblestones/normal`, `minTier` 0 | `recipe/ara_vitae/cobbled_deepslate.json` |

## Behaviour read from the repository

- The altar is one block entity. Every tick it looks at its single slot; if the
  stack matches an `AraVitaeRecipe` whose `minTier` is at most the structure's
  tier, it advances `progress` by `craftSpeed * (1 + consumption)` LP a tick,
  taken from the main tank, up to `bloodNeeded * count`. With no LP it bleeds
  `progress` back down by `drainSpeed * efficiency` a tick. At the target it
  replaces the slot's stack with the output (`AraVitaeTile.tickRecipeCrafting`,
  `completeCrafting`).
- `weak_blood_orb` has `drainSpeed` 0: an unfed altar keeps what it has built.
  The second recipe carried here (`cobbled_deepslate`, 50 LP) exists to show the
  table is data, not a special case.
- Right-clicking the altar with an item puts one item in the slot; right-clicking
  an occupied slot with an empty hand takes the stack
  (`AraVitaeBlock.useItemOn`).
- A bound orb right-clicked near an altar (`use`) trades health for LP:
  2 health → `2 * 100 = 200` LP, credited to the altar's main tank, overflow to
  the reserve. In creative it fills the altar's remaining room instead
  (`BloodOrbItem.use`, `AraVitaeTile.addSacrificeEV`).
- Structure: the tier is the **highest** validator that matches
  (`AltarUtil.getTier`); runes are counted only at the layout's `upgrade`
  positions (`AltarUtil.scanForRunes`).
- Tier 0 also leeches nearby players when a tier-0 recipe lacks LP
  (`AraVitaeTile.leechNearbyPlayers`) — **not** in this slice (see the gap list).

## Where the rewrite differs

- **The slot move is applied by the altar's tick.** At this pin a block-entity
  item slot written inside a use hook is not kept: the write is visible to a
  saved-compound read in the same hook, but the slot reads empty on the next
  query, while the same write from `perform_command` or from
  `block_entity_tick` persists. So `use_item_on`/`right_click_block` queue the
  move and the tick applies it — one tick of latency, invisible to a player.
  Upstream's `useItemOn` moves the stack inline.
- **A take is ignored for 500 ms after an insert on the same altar.** Claiming
  the click makes the engine retry it with the other hand, and that retry would
  take back what the first hand just inserted (upstream's insert answers SUCCESS,
  so no retry follows there).
- **One slot, no menu.** Upstream's altar is an `ItemStackHandler` with a GUI-less
  slot; the port declares one container slot with no menu, so hoppers and the
  harness reach it as upstream's handler does and a right-click still goes to
  `use_item_on`/`right_click_block`.
- **Self-sacrifice is queued.** The pin has no helper that hurts a player, so
  the orb's survival branch records a request that the altar's next tick settles
  with `/damage <player> 2 minecraft:magic` and credits the health actually lost
  (upstream's `takeSacrifice` returns the same measured value). Creative fills
  the altar's room directly, exactly as upstream's `instabuild` branch.
- **No Anima network.** The orb has no binding/anima pool yet; the orb is a
  plain altar filler. The network, the orb's internal tank and the orb-in-slot
  filling are gaps.
- **Two tiers.** Only `weak` (tier 0) and `apprentice` (tier 1) layouts are
  carried; `mage`+ need pillars and capstones from blocks outside the slice.
- **Data-driven structures.** The layouts are Rust constants transcribed from the
  two jar JSON files (recorded above) rather than a new datapack registry.
