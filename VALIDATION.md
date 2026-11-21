# Neo Vitae slice 1 — verification evidence

> **2026-11-18 (0.2.0):** this file is the **0.1.0 `.hnmod` and 2026-11-11 repair record**, kept as
> history: those binaries no longer load on launcher ≥ 0.2.70. The 0.2.0 SDK ladder — the package,
> its tests, `check`/`fits`, the hash and the entries, and the gap that the in-game leg needs a
> launcher-built game — is in `notes.md` under `## Migration to the SDK format (0.2.0, 2026-11-18)`
> and in `ports/_shared/evidence/neovitae/sdk-*.log`. The in-game logs named below
> (`e2e-embedded.log`, `e2e-selftest.log`, `e2e-repair-2026-11-11.log`) are historical evidence for
> the behaviour, not a run of this package.

Every command below ran through the campaign's build-slot registry
(`ports/tools/build-slot.sh`), which forwards the compile steps to the second
machine. Logs are in `ports/_shared/evidence/neovitae/`.

## 1. Pure runtime (standalone crate)

```
cargo fmt --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
```

`final-runtime.log`: **18 passed; 0 failed** (plus 0 doc-tests); clippy and fmt
clean, exit 0. A red run before the implementation is `red-runtime-tests.log`
(14 failed, 3 passed), then green in `green-runtime-tests.log`.

## 2. Staged build ladder (pin `1d63e0a5`, mixins `5a64af93`)

| Step | Log | Result |
|---|---|---|
| `tools/horizon.py stage` | `ladder-stage.log` | staged, `1d63e0a5…` |
| `tools/horizon.py check` | `ladder-check.log` | `cargo_exit 0`, **0 warnings** |
| `tools/horizon.py test` (`game::neovitae::tests`) | `ladder-embedded-test.log` | **18 passed**, 1240 filtered, exit 0 |
| `tools/horizon.py build` (fast) | `ladder-build.log` | `cargo_exit 0`, `horizon-client` fast binary |
| `tools/package.sh` | `package.log` | `.hmod` + `.hnmod`, both `horizon-mod check` green |

## 3. Packages (delivered in the port, not `target/`)

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/neovitae.hmod` | 20,913 B | `9f241738fc48c6dcfb9cd3d0ce6552b7c02909ff29514ce9cfe55cdbbc32e78d` | 40 |
| `neovitae.hnmod` | 33,165 B | `e187287c1a1962fa2a69466814aa1e59775e2a6f1c2b1cf95b55f4d9c8a9bae8` | 15 |

`horizon-mod check neovitae.hnmod --kind native-mod`:
`format native-1/1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9/5a64af938ec86b00d53d16a408aabc0038fb3e57, for 26.3`.

**Reproducible:** packaged twice from the same sources; both runs printed these
exact sizes and SHA-256s (`sha256sum` of the two runs was identical).

The `.hnmod` embeds the built content mod at
`mixins/neovitae/data/neovitae-content.hmod` (bytes identical to
`content/neovitae.hmod`: `cmp` clean, and the game's own startup copy was
compared byte-for-byte by the e2e below).

## 4. In-game e2e (`tools/e2e.sh`)

`build-slot.sh e2e --job NV-E17-final --port ports/neovitae -- tools/e2e.sh`:
**PASS + EXIT:0** (`e2e-embedded.log`). The run is the **embedded** leg — the game
is launched with **no `--dev-mod`**, so the block, item and recipes come only from
the content mod inside the `.hnmod` — and it includes a control launch of the same
binary with `.horizon-cache` made unusable, where the ids must be unknown.

Transcript points (each an assertion on a parsed `ctl` reply, not on the absence
of an error):

| Leg | What was checked |
|---|---|
| control (no embedded content) | `neovitae:ara_vitae` is not an item and `setblock` leaves `minecraft:air`; `.horizon-cache` untouched |
| embedded content | the bundle wrote `neovitae-embedded-content.hmod`, byte-identical to `content/neovitae.hmod` (20,913 B) |
| A | altar placed from its item: `tier 0`, `lp 0/10000` |
| B | eight runes at `y-1`: `tier 1`, `lp 0/18000` (only the four `upgrade` positions count) |
| C | creative orb click: `lp 18000/18000` (instabuild fills the room) |
| D | survival orb click from 0: `lp 200/18000`, health fell (200 LP = 2 points × 100) |
| E | right click with a diamond: slot `minecraft:diamond x1`, hover counters `uses 1 / right 2`; 30 ticks later still there with `lp 0` (dry-altar control) |
| F | after 220 ticks at `lp 18000`: slot `neovitae:blood_orb_weak x1`, `lp 16000` (2,000 LP at 10 a tick = 200 ticks) |
| G | empty-hand right click: the orb is in the player's inventory, slot empty |
| H | orb click 28 blocks away: the altar still holds the 1,000 LP the harness set |

**Fail-capable:** `CONTROL=0 EMBEDDED=1 SELFTEST=1 tools/e2e.sh` flips D's
assertion to 201 LP and the run exits non-zero: `FAIL: the tank is 200, expected
201` + `EXIT:1` (`e2e-selftest.log`). `e2e-diagnose.log` is the run that found the
hook-write behaviour noted in `notes.md` (a FAIL + `EXIT:1` by design).

## 5. Gates

```
python3 ports/tools/check-e2e-logs.py neovitae --strict   # exit 0, logs=3 ok=3
python3 ports/tools/check-skills.py --notes ports/neovitae # SKILLS GATE: PASS
python3 ports/tools/upstream_ready.py neovitae             # blocked only on the ledger status
```

`upstream_ready.py` accepts the licence (MIT, rank 0), the delivered `.hnmod` in
the port root, the `Skills:` line, the PASS e2e and the pin; its only remaining
objection is `status is pending, not ported`, which is the orchestrator's ledger
row to change.

## 26.3 dialect sweep (2026-11-21) — 0.2.2

| Check | Command | Result |
|---|---|---|
| static dialect | `python3 ports/tools/check_261_dialect.py neovitae.hmod` | exit 0; pre-fix package exit 1 (`../_shared/evidence/neovitae/dialect-check-red.log`) |
| tests | `cargo test --offline` | exit 0 (`dialect-test.log`) |
| package | `cargo horizon-mod build` twice after `rm -f Cargo.lock` | `cmp`-identical, built against 0a875da |
| registry check | `cargo horizon-mod check neovitae.hmod` | **57,571 B**, SHA-256 `e0453f447ebc0db5e3111b7f237b03038ebfde43d26784875c01f1b042a26d57`, exit 0 |
| fits the pin | `cargo horizon-mod fits neovitae.hmod --build 0a875da` | `0a875da  made for`, exit 0 |
| the gate | `python3 ports/tools/verify-sdk-package.py neovitae` | 1/1 |
| in game | `tools/e2e-dialect.sh` | `PASS` + `EXIT:0`; `SELFTEST=1` `FAIL` + `EXIT:1`; `check-e2e-logs.py neovitae --strict` exit 0 |

Changed: 4 loot tables. Proof: rune_blank drops 1; no recipe carried (altar crafting is the port's own e2e); pre-fix red. Full record in [`notes.md`](notes.md) §"26.3 dialect sweep".
