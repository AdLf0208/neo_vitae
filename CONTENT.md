# `content/` — what is carried and what is written

Built from the ATM10 jar `ports/_shared/licence-cache/neovitae-1.21.1-1.1.51.jar`
(SHA-256 `4f7c55e2e0aef1f341ba7b79596e419d49ec34ed948b724ead4a6508037047ec`,
12,348,069 B; **MIT**, read from `META-INF/neoforge.mods.toml`). The jar stays in
`licence-cache/` as the licence and asset evidence.

## Carried byte-for-byte from the jar

| File | Note |
|---|---|
| `assets/neovitae/blockstates/{ara_vitae,rune_blank,rune_capacity,rune_capacity_augmented}.json` | the jar's own, 1.21.1 variants still valid in 26.3 |
| `assets/neovitae/models/block/ara_vitae.json` | the Blockbench model (13 elements, 4 display transforms) |
| `assets/neovitae/models/block/rune_{blank,capacity,capacity_augmented}.json` | cube + glow overlay |
| `assets/neovitae/models/item/{ara_vitae,rune_*}.json` | parent = the block model |
| `assets/neovitae/textures/block/{altar,rune_blank,rune_capacity,rune_capacity_e,rune_capacity_augmented,rune_capacity_augmented_e}.png` | the six textures the models name |
| `assets/neovitae/textures/item/blood_orb_weak.png` | the orb sprite |
| `data/neovitae/loot_table/blocks/ara_vitae.json` | already the 26.3 singular `condition` shape |
| `data/neovitae/loot_table/blocks/rune_{blank,capacity,capacity_augmented}.json` | written to the 26.3 shape from the jar's own tables |
| `assets/neovitae/lang/en_us.json` | the six keys used, the jar's strings plus two action-bar messages |

## Written here (decisions, not upstream files)

| File | What and why |
|---|---|
| `horizon-mod.toml` | id `neovitae`, API 0.2, `side = both`, `registries` |
| `data/neovitae/horizon/block/*.json` | altar template **`minecraft:carved_pumpkin`**: a horizontal `facing` block with **no menu and no use**, so a right click reaches `use_item_on`/`right_click_block`. Upstream's `strength(2.0F, 5.0F)` → hardness 2 / blast 5, stone sounds, nothing occluded, 16×12×16 collision and outline (upstream's `BOX`). Runes use **`minecraft:stone_bricks`** (full cube, pickaxe) |
| `assets/neovitae/items/*.json` | 26.3 item-model definitions pointing at `neovitae:item/*` |
| `data/neovitae/horizon/item/blood_orb_weak.json` | item template `minecraft:diamond`, `max_stack_size` 1 (upstream's `stacksTo(1)`) |
| `data/neovitae/horizon/creative_tab/neovitae.json` | the slice's five entries |
| `data/minecraft/tags/block/mineable/pickaxe.json` | the altar and runes are pickaxe-mineable |
| `data/c/tags/item/gems/diamond.json`, `data/c/tags/item/cobblestones/normal.json` | the convention tags the jar's two recipes name; the prepared root ships none, and an unknown tag is an empty ingredient |

## Deliberately not carried

- **The altar recipes as data.** They are a modded recipe type
  (`neovitae:ara_vitae_recipe`), which the content format cannot declare; the two
  carried ones live in the native half with the jar's numbers. The other 29 altar
  recipes need blocks outside the slice.
- **The crafting tree above the altar** (vitae stone, tabula chain, rune crafting,
  orb upgrades) and every other family's assets — see the gap list in `notes.md`.
- **The orb's fill variants** (`models/item/variant/blood_orb_weak_fill_*`): the
  fill level is the Anima/fluid state this slice does not implement, so the item
  has one model.
- **The guide book (Modonomicon) and advancements** — out of slice.
- **The rune glow light** (`neoforge_data.block_light` 15 in the rune models):
  carried in the model but Horizon draws it as an ordinary cutout layer; the
  emissive half is a gap.
