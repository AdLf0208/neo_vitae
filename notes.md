# Neo Vitae — port notes (slice 1: the Ara Vitae altar, Capacity Runes, the weak orb)

Ported from **Neo Vitae 1.1.51** (`ports/_shared/licence-cache/neovitae-1.21.1-1.1.51.jar`,
12,348,069 B, 6,356 entries; `META-INF/neoforge.mods.toml` `license="MIT"`,
`modId="neovitae"`). **MIT → licence rank 0**; local-only until publishing is
approved (ARCHITECTURE.md §8). Pin **`1d63e0a5`**
(`game.rs` `64813cf4…`), mixins **`5a64af93…`** (schema 6).

Status: **ported-partial** — the weakest altar loop (tanks, structure tiers 0–1,
Capacity Rune family, one tier-0 recipe, the orb's altar filling) is implemented
and verified in game; the gap list below is the rest of the mod.

**Package 0.2.0 (2026-11-18):** the port is now one flat SDK `.hmod` (`api = "0.2"`, native
code + carried content), since Horizon's mod-API merge retired `.hnmod`. `src/runtime.rs` is
byte-identical to the 0.1.0 file; only the glue, the manifest and the packaging were rewritten.
See [`## Migration to the SDK format`](#migration-to-the-sdk-format-020-2026-11-18) — it also
supersedes the closed submission **PR #856**, whose changelog text is there. Everything below
that describes `mixins/neovitae/`, the plan and the `.hnmod` is the **0.1.0 record** (its tree
now lives in `ports/_shared/evidence/neovitae/retired-hnmod-0.1.0/`).

**Why this mod now:** rank-0 `content`/`deep` core mod, a P0 that the queue offers
(`ports/_shared/queue/brief-neovitae.md`), with no unported dependency.

## Class + tier and the approach

- **Class:** `content`; **tier:** `deep` (`ports/ledger.csv`,
  `ports/_shared/source-map/atm10-sources.csv`).
- **Approach** (`horizon/docs/modpacks.md`, "Tiers (the API a port needs)"): a
  `deep` shape — a block entity with a server tick, a multiblock structure
  validator, a fluid-like LP tank, a menu-less item slot and an item that acts on
  the block. The content half (blocks, models, textures, loot, lang) is carried;
  the behaviour is the agent port. Nothing here is `data`-only, and the tier's
  entity/AI/renderer surface is deliberately out of the slice.
- **Carry-over vs agent port:** carry = the altar/rune blockstates, models,
  textures, loot tables and lang lines from the jar (`CONTENT.md`); agent port =
  the tiers, rune maths, tanks, crafting loop, clicks, orb and commands.

## What the slice does

- **The altar** `neovitae:ara_vitae` (content template `minecraft:carved_pumpkin`:
  a horizontal `facing` block with no menu, so a right click reaches the use
  hooks; hardness 2 / blast 5 / stone sounds; collision 16×12×16, occludes
  nothing). It has a block entity (`block_entity`, API 15), ticks
  (`block_entity_ticks` + `block_entity_tick`, API 15/17) and one slot
  (`block_entity_container`, API 16, no menu).
- **Structure → tier**, data-driven from the jar's two layouts: `weak` (the altar
  alone, tier 0) and `apprentice` (altar + 8 runes at `y-1`, tier 1). The tier is
  the highest layout that matches (`AltarUtil.getTier`), revalidated every 100
  ticks (`AltarConstants.STRUCTURE_CHECK_INTERVAL`). Only the layout's four
  orthogonal `upgrade` positions are scanned for runes, as upstream's
  `scanForRunes` does.
- **The LP tank**: `10,000 × capacityMod` mB main (`AraVitaeTile.getMainCapacity`
  = `BUCKET_VOLUME × 10 × mod`), `1,000 × mod` I/O, reserve `× 20`
  (`AltarConstants.RESERVE_CAPACITY_FACTOR`). A capacity rune adds `0.2`, an
  augmented capacity rune multiplies by `1.075` (`data_maps/block/altar_rune_stats.json`,
  `AltarConstants.CAPACITY_PER_RUNE`/`AUGMENTED_CAPACITY_POWER`).
- **The crafting loop** (`tickRecipeCrafting`): with a matching input in the slot
  the altar draws `craftSpeed × (1 + consumption)` LP a tick into `progress`, up
  to `bloodNeeded × count`; with no LP it bleeds `progress` back by
  `drainSpeed × efficiency`; at the target the slot becomes the output (same
  count) and a 30-tick cooldown starts (`CRAFTING_COOLDOWN_TICKS`). Two tier-0
  recipes are carried with the jar's numbers: **weak orb** (`#c:gems/diamond →
  neovitae:blood_orb_weak`, 2,000 LP, speed 10, drain 0) and **cobbled deepslate**
  (`#c:cobblestones/normal`, 50 LP, speed 5, drain 0). A recipe with a tier above
  the structure's does not match.
- **Clicks** (`AraVitaeBlock.useItemOn`): a right click with an item puts one item
  in the slot; an empty-hand right click takes the stack back.
- **The orb** (`BloodOrbItem.use`): a right click with `neovitae:blood_orb_weak`
  within 5 blocks of an altar trades health for LP. Creative fills the altar's
  remaining room (upstream's `instabuild` branch, verified at exactly 18,000);
  survival pays `2 health × 100 = 200 LP` (`ServerConfig.SELF_SACRIFICE_CONVERSION`
  default), measured the way upstream's `takeSacrifice` measures it.
- **Harness controls**: `/neovitae altar <pos>` (tier, tank, reserve, progress,
  slot, hook counters, game time) and `/neovitae lp <pos> <amount>`,
  `/neovitae insert <pos> <item>` (operator). Upstream has no commands; precedent
  `/charginggadgets info|energy`, `/powah info|set`.

## Hooks (0.1.0 record — the retired `.hnmod` wire; the 0.2.0 table is in the migration section)

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15) | `block_entity.rs` | the altar's block entity |
| `block_entity_ticks` (15) | `block_entity_ticks.rs` | ticks on the server |
| `block_entity_container` (16/17) | `block_entity_container.rs` | one slot, every face, no menu |
| `block_entity_tick` (17) | `block_entity_tick.rs` | structure, queued clicks, sacriﬁce, crafting |
| `use_item_on` (13) | `use_item_on.rs` | insert one item |
| `use_item` (13) | `use_item.rs` | the orb's health → LP |
| `right_click_block` (21) | `right_click_block.rs` | empty-hand take-out |
| `register_commands` (8) | `register_commands.rs` | `/neovitae …` |
| `perform_command` (8) | `perform_command.rs` | `/neovitae …` |
| `content_root` (HEAD) | `embedded_content.rs` | the embedded `.hmod` (AE2 pattern) |

`horizon-native.json` declares `"hooks"` implicitly through the plan; the API
level used is 21 (the highest named hook).

## Packaging (0.1.0 record — retired with `.hnmod`)

One deliverable, `ports/neovitae/neovitae.hnmod`, carrying the built content mod
at `mixins/neovitae/data/neovitae-content.hmod` and written out at startup by the
`content_root` injection (AE2 pattern, as `charginggadgets`/`ae2` ship it). The
content package also stands alone in `content/neovitae.hmod`. Sizes and SHA-256s
are in `VALIDATION.md`.

## Deviations (each with its reason)

1. **The slot move is applied by the altar's tick, not inside the click.** A
   block-entity item slot written from inside a use hook is not kept at this pin
   (the write is visible to a saved-compound read in the same hook, but the slot
   reads empty on the next query; the same write from `perform_command` or from
   the tick persists — measured while building this slice). The click therefore
   queues the move and the tick applies it one tick later, which a player cannot
   see. Upstream's `useItemOn` moves the stack inline.
2. **A take is ignored for 500 ms after an insert on the same altar.** When the
   click is claimed the engine retries it with the other hand, and that retry
   would take back the item the first hand just inserted (upstream's insert
   answers SUCCESS, so no retry follows there).
3. **Self-sacrifice is queued and paid with `/damage`.** The pin has no helper
   that hurts a player, so the survival orb branch records a request that the
   altar's next tick settles with `/damage <player> 2 minecraft:magic` and credits
   the health actually lost (200 LP = 2 points × 100). Creative fills the room
   directly, as upstream's `instabuild` branch does, with no damage at all.
4. **Two tiers only.** `mage`+ need pillars and capstones from blocks outside the
   slice; their layouts are not carried, so the altar stops at apprentice.
5. **The layouts are Rust constants**, transcribed from
   `data/neovitae/neovitae/altar_tier/{weak,apprentice}.json`, not a new datapack
   registry (the pin has no consumer for the mod's own registry).
6. **The orb has no Anima binding.** The network, the orb's internal tank, the
   orb-in-slot filling and the blood shield are gaps; the orb here only pays the
   altar it stands next to.
7. **Tags are carried by this port.** `c:gems/diamond` and `c:cobblestones/normal`
   are convention tags the prepared 26.3 root does not ship; without them the two
   recipes could never match (an unknown tag is an empty ingredient).

## Gaps (the rest of the mod)

- Altar tiers `mage`/`master`/`archmage`/`transcendent` (pillars, t3–t6 capstones,
  tier effects), the 21-rune family beyond Capacity (speed, efficiency, sacrifice,
  self-sacrifice, acceleration, dislocation, orb, charging and their reinforced
  versions), the augmented-capacity *charging* capacity, the altar's fluid I/O
  (input/output/charging tanks), the reserve auto-move, the tier-0 bootstrap leech
  (`leechNearbyPlayers`), redstone signal/comparator output, the altar's particles
  and sounds, the block-entity renderer (the altar model is static here), the
  `.hmod` crafting tree above the altar (vitae stone, tabula rasa/robur/animata/
  spiritus/aetherea, rune crafting, the orb upgrades), the other altar recipes
  (29 more `ara_vitae` recipes), the orb items and their fluid/anima components,
  Anima/soul network, incense and tranquillity, Sentient armour and tools,
  sigils/anointments, rituals (197 dungeon recipes, 249 guide entries), Athanor,
  Alchemy Table, Hellfire Forge, Flasks, Arrays, Meteor, Teleposer, routing nodes,
  Spiritus, and worldgen/structures.
- No `[api]` shim is needed: every hook used is stamped at the pin.

## Upstream sources read (analysis aid)

- Repo <https://github.com/breakinblocks/NeoVitae>, commit
  `0abaa11f31add824a5d2be5018fbbe6a69b0a1bf` (2026-11-08), clone at
  `ports/_shared/upstream/neovitae` (analysis only; deleted after these notes).
- Files read: `common/blockentity/AraVitaeTile.java` (tick, tanks, calculateStats,
  tickRecipeCrafting, addSacrificeEV, completeCrafting), `AltarConstants.java`,
  `common/block/AraVitaeBlock.java`, `common/item/BloodOrbItem.java`,
  `util/AltarUtil.java` (getTier, scanForRunes, takeSacrifice,
  calculateSelfSacrificeLP), `ServerConfig.java` (self_sacrifice_conversion),
  `api/altar/rune/AltarRuneModifiers.java`, `common/datamap/AltarRuneStats.java`,
  and the jar's data: `altar_tier/*.json`, `data_maps/block/altar_rune_stats.json`,
  `data_maps/item/blood_orb_stats.json`, `recipe/ara_vitae/*.json`,
  `tags/block/altar/runes.json`, models/blockstates/textures/lang/loot.
- Behaviours extracted are in `FINDINGS.md`; the rewrite differs where the
  deviations above say.

## Verification (evidence in `ports/_shared/evidence/neovitae/`)

- `cargo test`: **18 passed** (standalone lib); `cargo clippy --all-targets
  -- -D warnings` and `cargo fmt --check`: clean (`final-runtime.log`).
- Staged `check`: `cargo_exit 0`, **0 warnings** (`ladder-check.log`).
- Embedded tests (`game::neovitae::tests`): **18 passed**, 1240 filtered
  (`ladder-embedded-test.log`).
- e2e (embedded bundle, no `--dev-mod`, plus the unusable-`.horizon-cache`
  control): **PASS + EXIT:0** (`e2e-embedded.log`); `SELFTEST=1`: FAIL + EXIT:1
  (`e2e-selftest.log`).
- Gates: `check-e2e-logs.py neovitae --strict` exit 0 (3 logs), `check-skills.py
  --notes ports/neovitae` PASS. `upstream_ready.py` reports only the ledger
  status (`pending` → the orchestrator's record) and the notes Skills line, which
  this file now carries.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-neovitae.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** item lost when a second insert replaces a pending one, and components dropped
  **Repair:** Fixed: the click reserves the real one-item stack (components included) and a replaced or uncommittable request is refunded — to the player, or to the altar's next tick if the inventory is full. `Request::Insert` now carries the `ItemStack` and its player; `one_of` keeps damage/enchantments. Tests: `pending_tests::a_second_click_replaces_the_pending_one_and_hands_it_back` plus the existing click e2e.
- **Reviewer:** queues keyed by XYZ only, not dimension
  **Repair:** Fixed: `AltarKey { dimension, pos }` keys the slot moves, the self-sacrifice queue and the insert-retry guard. Test: `the_same_coordinates_in_two_dimensions_do_not_share_work`.
- **Reviewer:** licence notice
  **Repair:** `LICENSE` + `LICENSE-NOTICE.md` in the `.hnmod`, `data/neovitae/license/` in the carried `.hmod`; the LICENSE names Saereth and WayofTime.
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

### Repair verification (2026-11-11/12)

- Staged `check`: check exit 0 (`ports/_shared/evidence/repair-2026-11-11/neovitae-check.log`)
- Embedded tests: embedded **21 passed; 0 failed** (`ports/_shared/evidence/repair-2026-11-11/neovitae-test.log`, the three new queue tests included)
- In-game: e2e **PASS** + `EXIT:0` (`ports/_shared/evidence/repair-2026-11-11/neovitae-e2e.log`): insert by click, craft, take by empty hand, out-of-range orb
- `cargo test` (standalone lib), `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` are clean in the port; the artifacts were built twice and
  hash identically (`registry.toml` carries the sha256 above).
- Skills: superpowers/using-superpowers,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices,rust-port


## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) **retired `.hnmod`**: the registry closed every
`.hnmod` submission on 2026-11-18 with one reason, and an `.hnmod` no longer loads. This port's former
submission, **PR #856** (`neovitae` 0.1.0), was closed by that purge. **0.2.0** is the same behaviour
republished as one flat SDK `.hmod`; the changelog text is at the end of this section.

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18, `package_format` 6,
`content_api` "0.2"), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78,
`cargo-horizon-mod` from `horizon-rust/horizon@main`. The pinned SDK is cached in-tree at
`../_shared/sdk/0b14f8a/` and copied into `.horizon/sdk/` (gitignored) for offline builds.

**Done in this order** (the campaign's rule, and the brief's ladder):

1. every retired path was **moved** into `_shared/evidence/neovitae/retired-hnmod-0.1.0/` *before*
   the first `cargo horizon-mod build` (the tool refuses a root holding non-package paths), and the
   old `content/` was lifted to the package root;
2. `src/runtime.rs` was copied with `cp`, proved with `cmp` + `sha256sum`, and `cargo test` ran on
   the new crate **before any glue existed**: **21 passed**, 0 failed (`sdk-runtime-first.log`);
3. the glue was written test-first: `sdk-test-red.log` is the run with the glue's functions still
   `todo!()` — **22 passed, 6 failed**, every failure the "not yet implemented" of one of them — and
   then implemented to green.

### What moved, and where the retired tree is

| 0.1.0 path | Now | Evidence |
|---|---|---|
| `mixins/neovitae/runtime.rs` (754 lines, 21 tests) | `src/runtime.rs`, **byte-identical** (`567622c6…a0a14`) | `retired-hnmod-0.1.0/mixins/neovitae/runtime.rs` |
| `mixins/neovitae/support.rs` (741 lines) + 9 hook helpers | `src/support.rs` (glue) + `src/lib.rs` (the one `Hooks`) | `retired-hnmod-0.1.0/mixins/neovitae/` |
| `mixins/neovitae/horizon-mixin.toml` (2 `[[module]]`, 10 `[[injection]]`) | no plan: `Registrar` declarations and typed `Hooks`/Part methods | `retired-hnmod-0.1.0/mixins/` |
| `mixins/neovitae/embedded_content.rs` + `data/neovitae-content.hmod` (AE2 pattern) | gone: the package carries `assets/` and `data/` itself | `retired-hnmod-0.1.0/mixins/` |
| `content/horizon-mod.toml` (id/name/version/api/side/capabilities) | merged into the root `horizon-mod.toml`, which adds `authors`, `description` and `[native]` | `retired-hnmod-0.1.0/content/` |
| `content/assets/`, `content/data/` | `assets/`, `data/` at the package root | compared file by file, below |
| `content/neovitae.hmod` (42,502 B, the content carry) | not needed: content is inside the one package | `retired-hnmod-0.1.0/content/` |
| `neovitae.hnmod` (69,509 B, embedded content) | `neovitae.hmod` (56,484 B, native + content) | `retired-hnmod-0.1.0/` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml`; the pinned commit decides the toolchain | `retired-hnmod-0.1.0/` |
| `tools/horizon.py`, `package.py`, `package.sh`, `stamp_license.py` | deleted: staging, `.hnmod` packaging and the licence stamp belong to the retired format (the latter two licence files are now carried directly) | `retired-hnmod-0.1.0/tools/` |
| `Cargo.lock` | not kept: it is packed and changes the artifact | not kept |

**Kept unchanged:** `tools/e2e.sh` (updated for a launcher-built game, below), `tools/parse_info.py`,
`notes.md`, `README.md`, `LICENSE`, `LICENSE-NOTICE.md`, `CONTENT.md`, `FINDINGS.md`,
`VALIDATION.md`.

### File map (0.2.0)

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id = "neovitae"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to `0b14f8a`/`e59da90` |
| `Cargo.toml` | one crate, `edition = "2024"`; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`: `register`, `register_commands`, `run_mod_command`, and the hook→Part table |
| `src/support.rs` | the glue: the altar's block entity Parts, both clicks, the tick, the structure scan, the orb, the queues, the commands. 7 unit tests of the declarations and the saved-compound codec |
| `src/runtime.rs` | the pure truth, **byte-identical**, 21 tests |
| `assets/` (27 files), `data/` (16 files) | the carried content (blockstates, models, textures, items, blocks, loot, tags, lang, creative tab, licence copies) |
| `icon.png` | 869 B, the published icon |
| `LICENSE`, `LICENSE-NOTICE.md` | MIT text and provenance, **inside** the package |
| `tools/e2e.sh`, `tools/parse_info.py` | the in-game scenario (needs a launcher-built game) and its readout parser |
| `neovitae.hmod` | the deliverable |

### The wire: every `[[module]]` and `[[injection]]`, and where it went

The two modules (`runtime` → `crate::game::neovitae`, `state` → `crate::game::neovitae_state`) are
gone as a mechanism: `src/runtime.rs` and `src/support.rs` are ordinary crate modules. `uses.json`
in the package lists the hooks and methods the build actually recorded.

| 0.1.0 injection (hook, API) | 0.2.0 |
|---|---|
| `embedded-content` (`content_root` HEAD, cancellable, AE2 pattern) | **none** — the flat `.hmod` carries the content; there is nothing to write out |
| `block-entity` (`block_entity` 15, returned the initial compound) | the block entity **type** declared in `Hooks::register` (`block_entity_type("ara_vitae").blocks(&["ara_vitae"])`) |
| `block-entity-ticks` (`block_entity_ticks` 15) | `BlockEntityPart::block_entity_ticks` on `support::AltarBlock` → `event.ticks = true` |
| `block-entity-container` (`block_entity_container` 16/17, one slot) | `Registrar::block_container(&altar, container())`: one slot, every face, **no menu** |
| `block-entity-tick` (`block_entity_tick` 17, `game`/`pos`/`entity`) | `BlockEntityPart::block_entity_tick` → `support::tick` (`event.pos`, `event.entity.nbt`/`.items`, the checked view) |
| `use-item-on` (`use_item_on` 13, `*claimed = true`) | `BlockPart::use_item_on` → `Some(InteractionResult::Success)` |
| `use-item` (`use_item` 13, `*claimed = true`) | `ItemPart::use_item` on `support::Orb` → `Some(InteractionResult::SuccessServer)` |
| `right-click-block` (`right_click_block` 21, `*canceled = true`) | `BlockPart::use_without_item` on `support::AltarBlock` (vanilla calls it directly for an empty hand) — the global hook is not needed |
| `register-commands` (`register_commands` 8) | `Hooks::register_commands` → `support::declare_commands(event.commands)` |
| `perform-command` (`perform_command` 8, the raw line split on whitespace) | `Hooks::run_mod_command` → `support::run_command` with typed `CommandInvocation` arguments; `Flow::Cancel` = handled |

Host calls translated one for one (the 0.1.0 `Game`/`world` reaches are gone):
`native_block(pos)` → `level.block(host, pos)` + `key.id(host)`; `native_dimension()` →
`level.dimension().id(host)` / `click.dimension.key().id()`; `native_with_mod_block_entity` →
`host.block_entity(level, pos)` → `data` / `merge_data` / `item_slot` / `set_item_slot`;
`native_register_commands`/`NativeCommand` → `Command::new`/`Command::op`; `native_feedback` →
`host.commands().feedback(Text::literal(…))`; `native_run_command` → `host.commands().run_command`;
`native_server_item_in_tag` → `host.tags().key` + `is_in_tag`; `native_server_players`/health →
`host.player(uuid)` + `player.name(host)` + `host.living(player.entity())` + `living.health(host)`;
`native_give_stack` → `ItemPlace::PlayerHand.insert` then the player's inventory slots;
`game.world.game_time` → `level.game_time(host)`; `game.world.modified.insert` → the tick's own
`Changed` reply.

### Manifest mapping

- `[mod] id`/`version`/`priority` → `id`/`version = "0.2.0"`, and no priority (ordering would be
  `#[listener(priority = …)]`; this port needs none).
- `[compatibility] horizon_revision` → `[native] horizon` (exact 40 hex), plus `mixins` **moved**
  from `5a64af93…` to `e59da909…` and `game = "26.3"`.
- `license`, `homepage`, `issues` are not fields: `build` strips them, so the manifest on disk is
  not the manifest in the package (the packaged one is what `check` reads).
- `capabilities = ["registries"]` is the schema's one value; the package really does declare blocks,
  items, a block entity type, loot, tags and a creative tab.

### The carried content: compared, not assumed

`sdk-carried-content.log` diffs every `assets/` and `data/` file against the retired
`content/neovitae.hmod` by sha256: **41 carried entries, 41 present, 40 byte-identical, 0 missing**.
The one difference is `data/neovitae/license/notice.txt`, rewritten because the retired text
described the `.hnmod` layout (`mixins/neovitae/LICENSE`, `stamp_license.py`); it is byte-identical
to the root `LICENSE-NOTICE.md`, as `license.txt` is to `LICENSE`. The retired package's other
entry, its `horizon-mod.toml`, was merged into the root manifest.

### The ladder (all in one build slot each, `CARGO_INCREMENTAL=0`, no `--target-dir`)

There is **no staging step** for an SDK package: the crate is compiled in place against
`.horizon/sdk/`. `build-slot.sh` still speaks the retired tool, so its step names only size the
threads; every command ran `cd ports/neovitae && …` with `BUILD_LOCAL=1` (the far box has no copy of
this port).

| Step | Command | Result |
|---|---|---|
| runtime only, before glue | `cargo test --offline` | **21 passed**, 0 failed — `sdk-runtime-first.log` |
| glue red on `todo!()` | `cargo test --offline` | **22 passed, 6 failed** (each failure "not yet implemented") — `sdk-test-red.log` |
| tests | `cargo test --offline` | **28 passed**, 0 failed (21 `runtime::`, 7 `support::`) — `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings — `sdk-clippy.log` |
| format | `cargo fmt --check` (after `rustfmt --edition 2024 src/lib.rs src/support.rs`) | clean — `sdk-fmt.log`, `sdk-fmt-first-run.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `SDK 0b14f8a … in ./.horizon/sdk`, `Built ./target/horizon-mod/neovitae.hmod` — `sdk-build.log` |
| registry check | `cargo horizon-mod check neovitae.hmod` | `neovitae 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `56484 bytes, SHA-256 517b6981ed7b47c12cec953375ee7f4476320981852039d41f0b179d097ada6f, unsigned`, **EXIT 0** — `sdk-check.log` |
| fits the pinned build | `cargo horizon-mod fits neovitae.hmod --build nightly` | `Neo Vitae:` / `0b14f8a  made for`, **EXIT 0** — `sdk-fits.log` |
| second build | `rm -f Cargo.lock && cargo horizon-mod build` again | same 56,484 B, same sha256, `cmp` against the delivered file identical — `sdk-build.log` |
| entries | `unzip -l neovitae.hmod` | **51 entries**: root `horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `icon.png`, `uses.json`; `src/{lib,runtime,support}.rs`; `assets/neovitae/…` (27); `data/{c,minecraft,neovitae}/…` (16). **No `Cargo.lock`** — `sdk-entries.log` |
| runtime untouched | `unzip -p neovitae.hmod src/runtime.rs \| sha256sum` | `567622c6…a0a14` = the retired file and `src/runtime.rs`, byte for byte — `sdk-sha256.txt`, `sdk-runtime-identical.log` |

`uses.json` (extracted from the package, `sdk-uses.json`, summarised in `sdk-uses-summary.txt`) is the
migration's best summary: **381 surface entries** — `made_for 0b14f8a`, **26 hook entries** over the
11 hooks the build recorded (`block_entity`, `block_entity_tick`, `block_entity_ticks`, `use_item_on`,
`use_without_item`, `use_item`, `register_commands`, `run_mod_command`, plus the three registration
events `blocks`/`items`/`command`), 39 methods (`Hooks::register`, `BlockPart::use_item_on`,
`BlockPart::use_without_item`, `BlockEntityPart::{block_entity,block_entity_ticks,block_entity_tick}`,
`ItemPart::use_item`, `DefaultCapabilitiesRegistrar::block_container`, `BlocksRegistrar::block_key`,
`ItemsRegistrar::item_key`, `Commands::{feedback,run_command}`, the host reads), 98 handle methods,
28 traits, 181 types, 5 part kinds, both macros (`horizon_mod`, `horizon_part`) and the one
`registries` capability.

### Traps this migration must pass on (each cost a step here)

1. **Move the retired paths before the first build.** `cargo horizon-mod build` accepts package paths
   only: `mixins/`, `content/`, `compatibility.json`, `rust-toolchain.toml`, `neovitae.hnmod` and
   the old tools all moved to `_shared/evidence/neovitae/retired-hnmod-0.1.0/` first. The retired
   tree is evidence; nothing was deleted (except `Cargo.lock`, which must never be packed).
2. **`Registrar::block_container` takes `&BlockKey`** while `block_part` and `block_entity_part` take
   it by value: `r.block_container(altar, …)` is `E0308 expected &BlockKey, found BlockKey`.
3. **Three host calls are handle reads, not `Host` methods**, and the compiler suggests lookalikes:
   `.transaction(…)` needs `use mod_api::HostTransaction`; `player_name` is `player.name(host)`;
   `block_key_id`/`dimension_key_id` are `key.id(host)` / `key.id(host)` on the *keys*.
4. **Two different dimension types.** A `BlockClick` carries a `LevelDimension` whose `.key()` is a
   `LevelKey` (`id()` → `&'static str`); a `LevelRef::dimension()` is a `DimensionKey`
   (`id(host)` → `Option<String>`). `AltarKey` takes the `&str` form, so the click and the tick
   convert differently and still key the same altar.
5. **A Part is attached to the altar itself**, so the 0.1.0 guards (`entity.id != ALTAR`,
   `game.native_block(pos) != ALTAR`) are gone rather than translated: `use_item_on`,
   `use_without_item` and both tick methods only ever run for this block.
6. **The empty-hand take is `use_without_item`, not `right_click_block`.** Vanilla calls
   `useWithoutItem` directly when the hand is empty, so the global hook is unnecessary; an empty
   `click.stack` in `use_item_on` answers `TryWithEmptyHand`, which keeps the route when the engine
   asks the block first.
7. **Reserve the item only after the queue key exists.** The extract from the hand comes after
   `altar_key(...)`; a `?` between them (the dimension id can be unavailable) would drop the item
   the hand just lost. Caught while writing, not in a run.
8. **`ItemResource::of(stack).to_stack(1)` is 0.1.0's `one_of`**: it keeps damage, enchantments and
   the stack's components, where `ItemStack::of(key)` keeps none of them.
9. **There is no "give anywhere" helper.** `host.give` is the client's creative one; the give-back
   merges into the clicked hand (`ItemPlace::PlayerHand`) and then into the inventory
   (`inventory_slots` + `set_inventory_slot`), exactly where 0.1.0's `native_give_stack` put it.
10. **`cargo fmt` must not run over `src/runtime.rs`.** `cargo fmt --check` diffs only `support.rs`
    here, but the safe move is `rustfmt --edition 2024 src/lib.rs src/support.rs`; the runtime's
    sha256 is `567622c6…a0a14` before and after.
11. **Three clippy stops at `-D warnings`:** `len() <= SLOT` with `SLOT == 0` is
    `absurd_extreme_comparisons` (use `is_empty()`), `if x.is_none() { return None }` is
    `question_mark`, and a `match` that only unwraps a `shrink` is the same.
12. **A fresh block entity starts with an empty compound, and here that is harmless but load-bearing**
    to state: `tier` reads 0 (weak) and the absent capacity multiplier defaults to 1.0, so a new
    altar reads as the weak altar with a 10,000 LP tank until the first 100-tick structure check —
    the same numbers 0.1.0's e2e asserted immediately. `support::a_fresh_compound_reads_as_the_weak_altar`
    pins it (unlike `naturesaura`, this port needs no initial-compound Part).
13. **The artifact changes when root documents do.** Adding the two licence copies under
    `data/neovitae/license/` and rewriting `README.md` moved the package from 54,146 B to 56,484 B;
    the "build twice, hash twice" pair must be the *last* two builds, not the first.

### Behaviour that changed with the format (deliberate)

- **The licence notice text now describes this layout** (`data/neovitae/license/notice.txt` and the
  root `LICENSE-NOTICE.md`); the MIT terms and the upstream copyright are unchanged.
- **The transports changed, the rules did not.** Every number, tier, recipe, tank and multiplier is
  still in the byte-identical `runtime.rs`; the 0.1.0 readout line (`/neovitae altar`) is kept field
  for field, so the existing assertions still match.
- **The embedded-content write-out is gone** (one `.hmod` is the content), so there is no
  `.horizon-cache/neovitae-embedded-content.hmod` to compare any more.
- **`tools/e2e.sh` now needs a launcher-built game** (`GAME=`) and takes an optional
  `CONTROL_GAME=` for the differential; the embedded/content control leg is retired with the format.
- The 0.1.0 deviations 1–7 above still stand unchanged (queued slot moves, the 500 ms take guard,
  `/damage` for the sacrifice, two tiers, layouts as Rust constants, no Anima binding, carried tags).

### Gaps

1. **No in-game re-run of 0.2.0.** The scenario needs a game the launcher built with this package
   (`horizon-launcher native-build ports/neovitae --instance … --trust-native-code`), and
   `horizon-launcher` is not installed here. The 0.1.0 runs remain the behaviour's evidence
   (`e2e-embedded.log`, `e2e-selftest.log`, `e2e-repair-2026-11-11.log`); what changed is the
   transport of the same calls, not the rules, which are byte-identical and tested.
2. **Unproven in game at 0.2.0:** the queued slot move written by the tick after a click. 0.1.0
   measured that a block-entity slot written *inside* a use hook was not kept at the old pin, which
   is why the move is queued; the SDK's checked tick view is what should persist it, and the in-game
   run is what would prove it.
3. **The orb's altar search reads 11×11×11 blocks per click** (1,331 `level.block` calls), unchanged
   from 0.1.0 and bounded by the 5-block range; a spatial index would be an engine feature.
4. **A crafted output whose item id is unknown is not written** (only reachable if the carried
   content failed to load); 0.1.0 wrote the stack regardless.
5. **A player who leaves between the orb click and the altar's tick pays nothing** (the uuid no
   longer resolves); 0.1.0 looked the player up by name and behaved the same.
6. The readout's `slots` field is the container declaration's count (1), as 0.1.0's
   `entity.items.len()` was.
7. Everything in the 0.1.0 gap list above still stands: mage and higher tiers, the other twenty
   runes, the altar's fluid I/O and reserve move, redstone/comparator output, particles, sounds, the
   block-entity renderer, the crafting tree above the altar, Anima, rituals and Sentinels.
8. The port is **local-only** until the maintainer approves publishing (MIT, rank 0); nothing in
   this migration changes that. `ports/ledger.csv` and `ARCHITECTURE.md` are outside this job's write
   ownership — the orchestrator records the 0.2.0 state.

No upstream defect surfaced in this migration; the `## Upstream defects fixed` rule therefore has
nothing new to add (the carried tags `c:gems/diamond` and `c:cobblestones/normal` remain the
compatibility fix recorded in deviation 7).

### Changelog (0.2.0) — supersedes PR #856

> **0.2.0 — republished as an SDK `.hmod`.** Horizon's mod-API merge (launcher ≥ 0.2.70) retired
> `.hnmod` and closed **#856** (`neovitae` 0.1.0); this version supersedes it. One flat
> `neovitae.hmod` now carries the code *and* the content (the altar and rune blocks, their items,
> loot, tags, lang, models, textures and creative tab): `src/lib.rs` is the one
> `impl mod_api::Hooks`, `src/support.rs` the glue, and `src/runtime.rs` is byte-identical to 0.1.0,
> so the altar's tiers, tanks, rune maths, crafting loop and the orb behave exactly as before.
> Built against SDK `0b14f8a` / mixins `e59da90` for 26.3. What changed for `api = "0.2"`: the
> mixin plan, the 10 injections, the 9 per-hook helper files and the embedded-content path are gone
> (a Part method is the block's own hook, `Registrar` declares the block entity type and the one-slot
> container, `run_mod_command` takes typed arguments); `/neovitae` keeps its exact readout line; the
> licence notice now lives at the package root and in `data/neovitae/license/`. Same feature set and
> the same gaps (mage and higher tiers, the other runes, fluid I/O, redstone, renderer, the crafting
> tree).

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
