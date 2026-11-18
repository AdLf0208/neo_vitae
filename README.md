# Neo Vitae — a Horizon port

**Neo Vitae** is a Minecraft mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [breakinblocks/NeoVitae](https://github.com/breakinblocks/NeoVitae) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Neo Vitae ([source](https://github.com/breakinblocks/NeoVitae)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `neovitae.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What works

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

## Not yet ported

- Altar tiers `mage`/`master`/`archmage`/`transcendent` (pillars, t3–t6 capstones,
- No `[api]` shim is needed: every hook used is stamped at the pin.
- **No in-game re-run of 0.2.0.** The scenario needs a game the launcher built with this package
- **Unproven in game at 0.2.0:** the queued slot move written by the tick after a click. 0.1.0
- **The orb's altar search reads 11×11×11 blocks per click** (1,331 `level.block` calls), unchanged
- **A crafted output whose item id is unknown is not written** (only reachable if the carried
- **A player who leaves between the orb click and the altar's tick pays nothing** (the uuid no
- The readout's `slots` field is the container declaration's count (1), as 0.1.0's

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `neovitae.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Neo Vitae was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/breakinblocks/NeoVitae

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run neovitae` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/neovitae/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
