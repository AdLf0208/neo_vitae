# Neo Vitae for Horizon — slice 1

A clean-room Horizon port of **Neo Vitae 1.1.51** (`breakinblocks/NeoVitae`,
MIT), Minecraft 26.3. This slice brings **the Ara Vitae blood altar** up as far as
the apprentice tier, with the Capacity Rune family, the altar's LP tank and
crafting loop, and the Novicius Orb of Vitae that fills it.

## What works

- **Ara Vitae** (`neovitae:ara_vitae`): a block entity with a 10,000 × capacity
  modifier LP tank (millibuckets), a 1,000 × modifier I/O tank and a × 20 reserve.
- **Structure tiers**: the altar alone is tier 0; the altar plus eight runes at
  `y-1` is tier 1 (apprentice). The tier is revalidated every 5 s; only the four
  orthogonal rune positions are scanned, as upstream does.
- **Capacity Runes** (`neovitae:rune_capacity`, `neovitae:rune_capacity_augmented`):
  +0.2 additive and × 1.075 compounding capacity. Four capacity runes on an
  apprentice altar: **18,000 LP** (an altar with no runes: 10,000).
- **Crafting**: a right click puts one item in the altar's slot; the altar draws
  LP a tick up to the recipe's cost and leaves the result. The jar's own numbers:
  a diamond (`#c:gems/diamond`) becomes a **Novicius Orb of Vitae** for **2,000 LP**
  at 10 LP a tick (200 ticks); cobblestone becomes cobbled deepslate for 50 LP.
  A dry altar leaves the item and the progress alone (`drainSpeed` 0).
- **The orb**: right-click it within 5 blocks of an altar. Creative fills the
  altar's remaining room; survival pays **200 LP for 2 health** (the jar's
  `self_sacrifice_conversion` of 100).
- **Taking the result**: an empty-hand right click takes the stack back.
- **Harness readouts**: `/neovitae altar|lp|insert` (upstream has no commands).

## Installing

The one deliverable is `neovitae.hmod` — a native Horizon mod for
**api 0.2**, code **and** content in one flat package: `horizon-mod.toml`,
`Cargo.toml`, `src/` (the `impl mod_api::Hooks` and the pure `runtime`), and the
carried `assets/` and `data/`. Build it with
`cargo horizon-mod build` against the SDK the manifest pins
(`0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, game 26.3), after which
`cargo horizon-mod check neovitae.hmod` and
`cargo horizon-mod fits neovitae.hmod --build nightly` are the package checks.
The launcher compiles the native source against its own copy of the same build;
nothing else is installed.

## Package layout

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity (`id`, `version = 0.2.0`, `api = "0.2"`, `side`, `capabilities`, `[native]` pins) |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from the fetched `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` |
| `src/support.rs` | the glue: the altar's block entity, its slot, both clicks, the tick, the orb, the commands |
| `src/runtime.rs` | the pure logic and its 21 tests — byte-identical to the retired `.hnmod` port's file |
| `assets/`, `data/` | the carried content (blockstates, models, textures, items, blocks, loot, tags, lang, creative tab, licence copies) |
| `tools/e2e.sh`, `tools/parse_info.py` | the in-game scenario and its readout parser (needs a launcher-built game) |
| `notes.md`, `FINDINGS.md`, `CONTENT.md`, `VALIDATION.md` | the port's record, the upstream behaviour, what was carried, and the ladder |

## Not in this slice

The mage/archmage/transcendent tiers, the other twenty rune types, the altar's
fluid I/O and redstone, Anima/soul networks, rituals, sigils, Sentient gear, the
Athanor, Alchemy Table, Hellfire Forge and the crafting tree above the altar.
`notes.md` carries the full gap list; the slice is submitted deliberately so the
community can continue it.

## Licence

The original mod is MIT (`breakinblocks/NeoVitae`); this port keeps the MIT terms
and the upstream copyright. See `LICENSE` and `LICENSE-NOTICE.md` (both inside
the package) and `CONTENT.md` for what was carried.
