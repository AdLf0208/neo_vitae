//! Game-side glue for Neo Vitae.
//!
//! [`crate::runtime`] is the pure truth: the altar's tiers and layouts, the LP tank and rune
//! arithmetic, the recipe table and the crafting loop, with its tests. This module is the only
//! place that reads the game and writes it back: the altar's block entity and its one slot, the
//! structure scan around it, the server tick, the two clicks, the orb and the readout commands.
//!
//! Upstream keeps the altar's state in its block entity's saved compound: the validated tier and
//! capacity multiplier, the main tank (`lp`), the reserve tank, the crafting progress and the
//! crafting cooldown. Its one slot (`Inv`) holds the input, or the finished output.
//!
//! Read from Neo Vitae 1.1.51 (`AraVitaeTile.tick`/`tickRecipeCrafting`/`calculateStats`/
//! `addSacrificeEV`, `AraVitaeBlock.useItemOn`, `BloodOrbItem.use`, `AltarUtil.getTier`/
//! `scanForRunes`); see `FINDINGS.md`. The mapping from the retired `.hnmod` hook helpers to the
//! typed hooks and Parts below is in `notes.md` under `## Migration to the SDK format`.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use mod_api::{
    BlockClick, BlockEntityPart, BlockEntityRef, BlockEntityTickEvent, BlockEntityTicksEvent,
    BlockPart, Command, CommandArgumentValue, Compound, Flow, GameMode, Hand, Host, HostSide,
    HostTransaction, Identifier, InteractionResult, ItemPart, ItemPlace, ItemResource, ItemStack,
    LevelRef, ModContainer, PlayerRef, Registrar, RunModCommandEvent, ServerHost, Tag, Text,
    UseItemEvent, UseItemOnEvent, UseWithoutItemEvent,
};

use crate::runtime::{
    self, ALTAR, Altar, AltarKey, BLOOD_ORB_WEAK, Craft, ORB_ALTAR_RANGE, ORB_SACRIFICE_HEALTH,
    Pending, SLOT, STRUCTURE_CHECK_INTERVAL, capacity_mod, recipe_for, sacrifice_lp, tier,
};

/// The altar's block entity type, in this mod's namespace (`neovitae:ara_vitae`).
const ALTAR_TYPE: &str = "ara_vitae";
/// How long after an insert a take at the same altar is ignored: the engine's other-hand retry
/// must not undo what the first hand just put in.
const RETRY_GUARD: Duration = Duration::from_millis(500);

// Saved-compound keys (upstream's `saveAdditional`, shortened).
const KEY_TIER: &str = "tier";
const KEY_CAP: &str = "cap";
const KEY_LP: &str = "lp";
const KEY_RESERVE: &str = "reserve";
const KEY_PROGRESS: &str = "progress";
const KEY_COOLDOWN: &str = "cooldown";
const KEY_RECIPE: &str = "recipe";
const KEY_CHECK: &str = "check";
/// Harness counters: how many clicks reached each hook on this altar.
const KEY_USES: &str = "uses";
const KEY_RIGHT: &str = "right";

/// The slot number as the SDK's block entity helpers take it.
const SLOT_U32: u32 = SLOT as u32;
/// The player's inventory slots a plain give fills: the hotbar and main inventory.
const INVENTORY_SLOTS: u32 = 36;

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------

/// What the game answers for the altar (`EntityBlock`, `RegisterCapabilitiesEvent` and
/// `Item.useItemOn`/`Item.use`): its block entity type, its one-slot container, the block Part that
/// takes the clicks and ticks, and the orb's item Part.
pub fn declare(r: &mut dyn Registrar) {
    let altar = r.block_key(ALTAR);
    let kind = r
        .block_entity_type(ALTAR_TYPE)
        .blocks(&[ALTAR_TYPE])
        .build();
    r.block_part(altar, AltarBlock);
    r.block_entity_part(&kind, AltarBlock)
        .expect("the altar's Part attaches to the type this mod just declared");
    r.block_container(&altar, container());
    let orb = r.item_key(BLOOD_ORB_WEAK);
    r.item_part(orb, Orb);
}

/// The altar's one slot, reachable from every face, with no menu: a right click still reaches the
/// use hooks, as upstream's GUI-less `ItemStackHandler` does.
fn container() -> ModContainer {
    ModContainer {
        slots: SLOT + 1,
        faces: std::array::from_fn(|_| vec![SLOT]),
        inputs: vec![SLOT],
        outputs: vec![SLOT],
        menu: None,
    }
}

/// `/neovitae altar|lp|insert`: the harness readouts and setters. Upstream has no commands; these
/// exist so a scenario can read and set an altar without a GUI (precedent `/charginggadgets
/// info|energy`).
pub fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::new("neovitae altar <pos:block_pos>"));
    commands.push(Command::op(
        "neovitae lp <pos:block_pos> <amount:integer>",
        2,
    ));
    commands.push(Command::op(
        "neovitae insert <pos:block_pos> <item:word>",
        2,
    ));
}

// ---------------------------------------------------------------------------
// The altar's Part
// ---------------------------------------------------------------------------

/// The altar's block Part: it ticks, and it answers the two clicks (`BlockBehaviour.useItemOn` and
/// `useWithoutItem`). Both are block hooks here, so they run only for the altar's own state.
pub struct AltarBlock;

#[mod_api::horizon_part]
impl BlockPart for AltarBlock {
    /// A right click with an item on the altar (`AraVitaeBlock.useItemOn`): one item goes into its
    /// slot. An occupied altar consumes the click, as upstream's `SUCCESS` does.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        insert(host, event.click)
    }

    /// An empty-hand right click on the altar takes its stack back (upstream's second branch).
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        take(host, event.click)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for AltarBlock {
    /// The altar ticks on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: settle a queued click, pay a queued sacrifice, revalidate the structure
    /// every five seconds, then run the crafting loop (`AraVitaeTile.tick`).
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        tick(host, event);
    }
}

// ---------------------------------------------------------------------------
// The clicks
// ---------------------------------------------------------------------------

/// A right click with an item on the altar puts one item in its slot (`AraVitaeBlock.useItemOn`).
///
/// The slot move itself is queued for the altar's next tick: 0.1.0 measured that a block entity's
/// item slots written inside a use hook are not kept at this pin's predecessor, while the
/// altar's own tick persists them; a player cannot see the one-tick difference. The item is
/// reserved now — taken out of the clicked hand, components and all — so a request that is
/// replaced or cannot commit is given back instead of dropped.
fn insert(host: &mut dyn ServerHost, click: &BlockClick<'_>) -> Option<InteractionResult> {
    let Some(stack) = click.stack.filter(|stack| stack.count() > 0) else {
        // An empty hand: an altar has nothing to put in, so the click goes on to
        // `use_without_item`, which is where the take lives.
        return Some(InteractionResult::TryWithEmptyHand);
    };
    let level = host.level(click.dimension.key().id())?;
    let pos = click.pos;
    count_click(host, level, pos, KEY_USES);
    let Some(key) = altar_key(&*host, level, pos) else {
        return Some(InteractionResult::Success);
    };
    // A missing block entity is treated as occupied, as 0.1.0's `unwrap_or(true)` did: nothing goes
    // into an altar whose storage the game does not hold yet.
    let Some(entity) = altar_entity(&*host, level, pos) else {
        return Some(InteractionResult::Success);
    };
    if entity.item_slot(&*host, SLOT_U32).ok().flatten().is_some() {
        // Upstream consumes the click: an occupied altar takes nothing more.
        return Some(InteractionResult::Success);
    }
    let resource = ItemResource::of(stack);
    let Some(one) = resource.to_stack(1) else {
        return Some(InteractionResult::Success);
    };
    let hand = ItemPlace::PlayerHand {
        player: click.player,
        hand: click.hand,
    };
    // A creative player's hand gives without losing any.
    let taken = host
        .transaction(|host, tx| Ok::<u32, ()>(hand.extract(host, tx, &resource, 1)))
        .unwrap_or(0);
    if taken == 0 {
        return Some(InteractionResult::Success);
    }
    let request = Request::Insert {
        stack: one,
        player: click.player,
        hand: click.hand,
    };
    if let Some(replaced) = queue(request, key.clone()) {
        refund(host, &key, replaced);
    }
    mark_insert(key);
    Some(InteractionResult::Success)
}

/// An empty-hand right click on the altar takes its stack back, applied by the altar's next tick
/// exactly as the insert is.
fn take(host: &mut dyn ServerHost, click: &BlockClick<'_>) -> Option<InteractionResult> {
    if click.stack.is_some_and(|stack| stack.count() > 0) {
        return None;
    }
    let level = host.level(click.dimension.key().id())?;
    let pos = click.pos;
    count_click(host, level, pos, KEY_RIGHT);
    let key = altar_key(&*host, level, pos)?;
    // The engine asks again for the other hand of one click; that retry must not undo the insert
    // it just asked for (upstream's insert answers SUCCESS, so no retry follows there).
    if inserted_recently(&key) {
        return None;
    }
    slot_stack(&*host, level, pos)?;
    let request = Request::Take {
        player: click.player,
        hand: click.hand,
    };
    if let Some(replaced) = queue(request, key.clone()) {
        refund(host, &key, replaced);
    }
    Some(InteractionResult::Success)
}

/// One click that reached a hook on the altar, for the readout.
fn count_click(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], key: &str) {
    let Some(entity) = altar_entity(&*host, level, pos) else {
        return;
    };
    let Some(mut data) = entity.data(&*host) else {
        return;
    };
    let count = read_int(&data, key).wrapping_add(1);
    write_int(&mut data, key, count);
    let _ = entity.merge_data(host, data);
}

// ---------------------------------------------------------------------------
// The orb
// ---------------------------------------------------------------------------

/// The weak orb's item Part: `BloodOrbItem.use`. Near an altar it trades health for LP. In creative
/// it fills the altar's room, as upstream's `instabuild` branch does; in survival the sacrifice is
/// queued and settled by the altar's next tick, because a player's health is not written from
/// inside their own item use.
pub struct Orb;

#[mod_api::horizon_part]
impl ItemPart for Orb {
    fn use_item(
        &self,
        host: HostSide<'_>,
        event: &mut UseItemEvent<'_>,
    ) -> Option<InteractionResult> {
        let HostSide::Server(server) = host else {
            return None;
        };
        if event.stack.item().id(&*server) != BLOOD_ORB_WEAK {
            return None;
        }
        let player = server.player(event.player)?;
        let level = player.level(&*server)?;
        let at = player.entity().position(&*server)?;
        let at = [
            at[0].floor() as i32,
            at[1].floor() as i32,
            at[2].floor() as i32,
        ];
        let altar_pos = find_altar(&*server, level, at)?;
        if player.game_mode(&*server) == Some(GameMode::Creative) {
            // Upstream: a creative player pays nothing and fills the room.
            let entity = altar_entity(&*server, level, altar_pos)?;
            let mut data = entity.data(&*server)?;
            let mut altar = read_altar(&data);
            let room = (altar.capacity() - altar.lp).max(0);
            altar.add_lp(room);
            let check = read_int(&data, KEY_CHECK);
            write_altar(&mut data, &altar, check);
            let _ = entity.merge_data(server, data);
            return Some(InteractionResult::SuccessServer);
        }
        let key = altar_key(&*server, level, altar_pos)?;
        queue_sacrifice(event.player, key);
        Some(InteractionResult::SuccessServer)
    }
}

/// The altar within `ORB_ALTAR_RANGE` of `at`, as `AltarUtil.findAltar` scans.
fn find_altar(host: &dyn Host, level: LevelRef, at: [i32; 3]) -> Option<[i32; 3]> {
    let altar = host.block_key(ALTAR)?;
    for dx in -ORB_ALTAR_RANGE..=ORB_ALTAR_RANGE {
        for dy in -ORB_ALTAR_RANGE..=ORB_ALTAR_RANGE {
            for dz in -ORB_ALTAR_RANGE..=ORB_ALTAR_RANGE {
                let pos = [at[0] + dx, at[1] + dy, at[2] + dz];
                if level.block(host, pos) == Some(altar) {
                    return Some(pos);
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// One server tick of the altar
// ---------------------------------------------------------------------------

/// One server tick (`AraVitaeTile.tick`): settle a queued click, pay a queued self-sacrifice,
/// revalidate the structure every five seconds, run the crafting loop, then store the result.
fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let Some(level) = level_in_place(host) else {
        return;
    };
    let Some(key) = altar_key(&*host, level, event.pos) else {
        return;
    };
    if event.entity.items.is_empty() {
        event.entity.items.resize(SLOT + 1, None);
    }
    settle_request(host, &key, &mut event.entity.items);
    let mut altar = read_altar(&event.entity.nbt);
    settle_sacrifice(host, &key, &mut altar);
    let check = read_int(&event.entity.nbt, KEY_CHECK).wrapping_add(1);
    if check % STRUCTURE_CHECK_INTERVAL == 0 || altar.tier < 0 {
        let (found, modifier) = scan(&*host, level, event.pos);
        altar.set_structure(found, modifier);
    }
    if altar.cooldown > 0 {
        altar.cooldown -= 1;
    }
    if let Some(stack) = event.entity.items[SLOT].clone()
        && altar.cooldown == 0
    {
        let recipe = recipe_for(&stack.item().id(&*host), altar.tier, |tag, item| {
            in_tag(&*host, item, tag)
        });
        if altar.craft_tick(recipe, i32::try_from(stack.count()).unwrap_or(1)) == Craft::Done
            && let Some(recipe) = recipe
            && let Some(output) = item_stack(&*host, recipe.output, stack.count())
        {
            event.entity.items[SLOT] = Some(output);
        }
    }
    write_altar(&mut event.entity.nbt, &altar, check);
}

/// The altar's tier and the capacity multiplier of the runes standing at its layout's `upgrade`
/// positions (`AltarUtil.getTier` + `scanForRunes` + `calculateStats`).
fn scan(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> (i32, f32) {
    let found = tier(|offset| block_id(host, level, add(pos, offset)));
    let Ok(index) = usize::try_from(found) else {
        return (found, 1.0);
    };
    let Some(layout) = runtime::TIERS.get(index).copied() else {
        return (found, 1.0);
    };
    let runes: Vec<String> = layout
        .iter()
        .filter(|component| component.upgrade)
        .filter_map(|component| block_id(host, level, add(pos, component.offset)))
        .collect();
    let borrowed: Vec<&str> = runes.iter().map(String::as_str).collect();
    (found, capacity_mod(&borrowed))
}

/// The level the server is ticking now: the one a block-entity tick runs in.
fn level_in_place(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|&level| level.in_place(host))
}

// ---------------------------------------------------------------------------
// Queued click work
// ---------------------------------------------------------------------------

/// A slot move a click asked for, applied by the altar's next tick.
enum Request {
    /// This one-item stack into the free slot, for this player (who is given it back if the slot
    /// is filled before the tick).
    Insert {
        stack: ItemStack,
        player: u128,
        hand: Hand,
    },
    /// The slot's stack into this player's hand.
    Take { player: u128, hand: Hand },
}

fn requests() -> &'static Mutex<Pending<Request>> {
    static REQUESTS: OnceLock<Mutex<Pending<Request>>> = OnceLock::new();
    REQUESTS.get_or_init(|| Mutex::new(Pending::default()))
}

fn queue(request: Request, key: AltarKey) -> Option<Request> {
    let mut waiting = requests()
        .lock()
        .expect("the altar's request queue is only locked for one push or take");
    waiting.push(key, request)
}

/// Gives back what an insert request had reserved: the item goes back into the clicked hand, then
/// into the player's inventory, or — when it does not fit, or the request recorded no player —
/// back into the altar's queue, so the next tick tries again. A take reserved nothing.
fn refund(host: &mut dyn ServerHost, key: &AltarKey, request: Request) {
    let Request::Insert {
        stack,
        player,
        hand,
    } = request
    else {
        return;
    };
    let leftover = if player == 0 {
        Some(stack)
    } else {
        give_back(host, player, hand, stack)
    };
    if let Some(stack) = leftover {
        let mut waiting = requests()
            .lock()
            .expect("the altar's request queue is only locked for one push or take");
        waiting.push(
            key.clone(),
            Request::Insert {
                stack,
                player,
                hand,
            },
        );
    }
}

/// Applies a queued insert or take for `key`'s altar (`AraVitaeBlock.useItemOn`'s slot moves).
fn settle_request(host: &mut dyn ServerHost, key: &AltarKey, items: &mut [Option<ItemStack>]) {
    let request = {
        let mut waiting = requests()
            .lock()
            .expect("the altar's request queue is only locked for one push or take");
        waiting.take(key)
    };
    let Some(request) = request else {
        return;
    };
    match request {
        Request::Insert {
            stack,
            player,
            hand,
        } => {
            if items[SLOT].is_none() {
                items[SLOT] = Some(stack);
            } else {
                // Another click filled the slot first: the reservation goes back to the player who
                // made it (or to this altar's next tick), never into nothing.
                refund(
                    host,
                    key,
                    Request::Insert {
                        stack,
                        player,
                        hand,
                    },
                );
            }
        }
        Request::Take { player, hand } => {
            let Some(stack) = items[SLOT].take() else {
                return;
            };
            if let Some(left) = give_back(host, player, hand, stack) {
                items[SLOT] = Some(left);
            }
        }
    }
}

/// Puts `stack` back with `player`: into the hand that clicked, then into the inventory. What does
/// not fit comes back for the caller to keep.
fn give_back(
    host: &mut dyn ServerHost,
    player: u128,
    hand: Hand,
    stack: ItemStack,
) -> Option<ItemStack> {
    let resource = ItemResource::of(&stack);
    let count = stack.count();
    let in_hand = host
        .transaction(|host, tx| {
            let place = ItemPlace::PlayerHand { player, hand };
            Ok::<u32, ()>(place.insert(host, tx, &resource, count))
        })
        .unwrap_or(0);
    if in_hand == count {
        return None;
    }
    give_inventory(host, player, resource.to_stack(count - in_hand)?)
}

/// Fills a player's inventory with `stack`: the stacks of the same item first, then the empty
/// slots. What does not fit comes back.
fn give_inventory(host: &mut dyn ServerHost, uuid: u128, stack: ItemStack) -> Option<ItemStack> {
    let player = host.player(uuid)?;
    let max = host.max_stack_size(stack.item()).unwrap_or(64).max(1);
    let mut left = stack;
    let mut taken: Vec<u32> = Vec::new();
    for (slot, here) in player.inventory_slots(&*host).unwrap_or_default() {
        taken.push(slot);
        if left.count() == 0 {
            break;
        }
        if !here.is_stackable_with(&left) {
            continue;
        }
        let room = max.saturating_sub(here.count());
        if room == 0 {
            continue;
        }
        let moved = room.min(left.count());
        let mut grown = here.clone();
        if grown.grow(moved).is_err() {
            continue;
        }
        if player.set_inventory_slot(host, slot, Some(grown)).is_err() {
            continue;
        }
        left = left.shrink(moved)?;
    }
    for slot in 0..INVENTORY_SLOTS {
        if left.count() == 0 {
            break;
        }
        if taken.contains(&slot) {
            continue;
        }
        let moved = max.min(left.count());
        let put = ItemResource::of(&left).to_stack(moved)?;
        if player.set_inventory_slot(host, slot, Some(put)).is_err() {
            continue;
        }
        left = left.shrink(moved)?;
    }
    Some(left)
}

// ---------------------------------------------------------------------------
// A queued self-sacrifice
// ---------------------------------------------------------------------------

fn sacrifices() -> &'static Mutex<Pending<u128>> {
    static SACRIFICES: OnceLock<Mutex<Pending<u128>>> = OnceLock::new();
    SACRIFICES.get_or_init(|| Mutex::new(Pending::default()))
}

fn queue_sacrifice(player: u128, key: AltarKey) {
    let mut waiting = sacrifices()
        .lock()
        .expect("the sacrifice queue is only locked for one push or take");
    // A second orb click before the tick replaces the first; the replaced click committed nothing,
    // so nothing has to be given back.
    let _replaced = waiting.push(key, player);
}

/// Pays a queued sacrifice: `/damage <player> 2 minecraft:magic` as the console, then credit the
/// health actually lost (upstream's `takeSacrifice` measures the same).
fn settle_sacrifice(host: &mut dyn ServerHost, key: &AltarKey, altar: &mut Altar) {
    let entry = {
        let mut waiting = sacrifices()
            .lock()
            .expect("the sacrifice queue is only locked for one push or take");
        waiting.take(key)
    };
    let Some(uuid) = entry else {
        return;
    };
    let Some(player) = host.player(uuid) else {
        return;
    };
    let Some(name) = player.name(&*host) else {
        return;
    };
    let before = health_of(&*host, player);
    let ran = host.commands().run_command(&format!(
        "damage {name} {ORB_SACRIFICE_HEALTH} minecraft:magic"
    ));
    let after = health_of(&*host, player);
    let lost = match (before, after, ran) {
        (Some(before), Some(after), true) => (before - after).max(0.0),
        _ => 0.0,
    };
    let credited = sacrifice_lp(lost.round() as i32);
    altar.add_lp(credited);
}

fn health_of(host: &dyn Host, player: PlayerRef) -> Option<f32> {
    let living = host.living(player.entity())?;
    living.health(host)
}

// ---------------------------------------------------------------------------
// The `/neovitae` harness controls
// ---------------------------------------------------------------------------

/// `/neovitae altar|lp|insert` (`Hooks::run_mod_command`). Upstream has no commands; the readout
/// text is kept byte-identical to 0.1.0's.
pub fn run_command(host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
    let invocation = event.command;
    let Some(root) = invocation.literals.first().map(String::as_str) else {
        return Flow::Continue;
    };
    if root != "neovitae" {
        return Flow::Continue;
    }
    let sub = invocation.literals.get(1).map(String::as_str);
    let level = invocation
        .dimension
        .id(&*host)
        .and_then(|id| host.level(&id));
    match sub {
        Some("altar") => {
            let Some(pos) = block_pos(invocation, "pos") else {
                say(host, "usage: /neovitae altar <x> <y> <z>");
                return Flow::Cancel;
            };
            let line = match level {
                Some(level) => readout(host, level, pos),
                None => format!("{pos:?} is not a mod block entity."),
            };
            say(host, &line);
        }
        Some("lp") => {
            let (Some(pos), Some(amount)) =
                (block_pos(invocation, "pos"), integer(invocation, "amount"))
            else {
                say(host, "usage: /neovitae lp <x> <y> <z> <amount>");
                return Flow::Cancel;
            };
            let Some(level) = level else {
                say(host, &format!("{pos:?} is not an Ara Vitae."));
                return Flow::Cancel;
            };
            let Some(entity) = altar_entity(&*host, level, pos) else {
                say(host, &format!("{pos:?} is not an Ara Vitae."));
                return Flow::Cancel;
            };
            let Some(mut data) = entity.data(&*host) else {
                say(host, &format!("{pos:?} is not an Ara Vitae."));
                return Flow::Cancel;
            };
            let mut altar = read_altar(&data);
            let capacity = altar.capacity();
            let wanted = amount.clamp(0, capacity);
            altar.lp = wanted;
            let check = read_int(&data, KEY_CHECK);
            write_altar(&mut data, &altar, check);
            // `merge_data` answers `Ok(false)` for data the entity already held, which is a
            // successful no-op; 0.1.0 reported the entity being ours, so only a refused write is
            // "not an Ara Vitae".
            let wrote = entity.merge_data(host, data).is_ok();
            let line = if wrote {
                format!("Ara Vitae at {pos:?}: main tank set to {wanted}/{capacity}.")
            } else {
                format!("{pos:?} is not an Ara Vitae.")
            };
            say(host, &line);
        }
        Some("insert") => {
            let (Some(pos), Some(item)) = (block_pos(invocation, "pos"), word(invocation, "item"))
            else {
                say(host, "usage: /neovitae insert <x> <y> <z> <item>");
                return Flow::Cancel;
            };
            let placed = level
                .and_then(|level| {
                    let entity = altar_entity(&*host, level, pos)?;
                    let stack = item_stack(&*host, item, 1)?;
                    host.transaction(|host, tx| {
                        entity
                            .set_item_slot(host, tx, SLOT_U32, Some(stack))
                            .map_err(|_| ())
                    })
                    .ok()
                })
                .unwrap_or(false);
            say(
                host,
                &format!("Ara Vitae at {pos:?}: insert {item} -> {placed}"),
            );
        }
        _ => say(host, "usage: /neovitae altar|lp|insert ..."),
    }
    Flow::Cancel
}

/// One feedback line to the command's source, vanilla's silence and track-output rules included.
fn say(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// The `pos` block-position argument.
fn block_pos(invocation: &mod_api::CommandInvocation, name: &str) -> Option<[i32; 3]> {
    match invocation.argument(name)? {
        CommandArgumentValue::BlockPos(pos) => Some(*pos),
        _ => None,
    }
}

/// A `word` argument.
fn word<'a>(invocation: &'a mod_api::CommandInvocation, name: &str) -> Option<&'a str> {
    match invocation.argument(name)? {
        CommandArgumentValue::String(word) => Some(word.as_str()),
        _ => None,
    }
}

/// An `integer` argument.
fn integer(invocation: &mod_api::CommandInvocation, name: &str) -> Option<i32> {
    match invocation.argument(name)? {
        CommandArgumentValue::Integer(value) => Some(*value),
        _ => None,
    }
}

/// The altar's state, as `/neovitae altar` prints it (0.1.0's line, field for field).
fn readout(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> String {
    let is_altar = host
        .block_key(ALTAR)
        .is_some_and(|altar| level.block(&*host, pos) == Some(altar));
    if !is_altar {
        return match host.block_entity(level, pos) {
            Some(_) => format!("{pos:?} is not an Ara Vitae."),
            None => format!("{pos:?} is not a mod block entity."),
        };
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return format!("{pos:?} is not a mod block entity.");
    };
    let Some(data) = entity.data(&*host) else {
        return format!("{pos:?} is not a mod block entity.");
    };
    let altar = read_altar(&data);
    let slots = entity.item_slot(&*host, SLOT_U32).ok().flatten();
    let slot = match slots {
        Some(stack) => format!("{} x{}", stack.item().id(&*host), stack.count()),
        None => "empty".to_owned(),
    };
    let time = level.game_time(&*host).unwrap_or(0);
    format!(
        "Ara Vitae at {pos:?}: tier {}, lp {}/{}, reserve {}, progress {}, slots {}, slot {}, clicks {}/{}, t={}",
        altar.tier,
        altar.lp,
        altar.capacity(),
        altar.reserve,
        altar.progress,
        container().slots,
        slot,
        read_int(&data, KEY_USES),
        read_int(&data, KEY_RIGHT),
        time
    )
}

// ---------------------------------------------------------------------------
// The altar's place in the world
// ---------------------------------------------------------------------------

/// The altar's key: the level it stands in and its position, so two altars at the same
/// coordinates in different dimensions never share queued work.
fn altar_key(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<AltarKey> {
    let dimension = level.dimension().id(host)?;
    Some(AltarKey::new(&dimension, pos))
}

/// The mod block entity at `pos`, when the block there is the altar.
fn altar_entity(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<BlockEntityRef> {
    let altar = host.block_key(ALTAR)?;
    if level.block(host, pos) != Some(altar) {
        return None;
    }
    host.block_entity(level, pos)
}

/// The altar's slot, read straight from its block entity.
fn slot_stack(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<ItemStack> {
    let entity = altar_entity(host, level, pos)?;
    entity.item_slot(host, SLOT_U32).ok().flatten()
}

/// The id of the block at `pos`, or `None` when that position is not loaded.
fn block_id(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    level.block(host, pos).map(|key| key.id(host))
}

/// A fresh stack of `id`, or `None` for an id this game does not know.
fn item_stack(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    let id = Identifier::parse(id).ok()?;
    let key = host.item_key(&id)?;
    ItemStack::new(key, count)
}

/// Whether `item` is in the world's item tag `tag` (`TagTools.hasTag`).
fn in_tag(host: &dyn Host, item: &str, tag: &str) -> bool {
    let Ok(tag) = Identifier::parse(tag) else {
        return false;
    };
    let Some(tag) = host.tags().key(&tag) else {
        return false;
    };
    let Ok(item) = Identifier::parse(item) else {
        return false;
    };
    let Some(item) = host.item_key(&item) else {
        return false;
    };
    host.tags().is_in_tag(item, tag).unwrap_or(false)
}

fn add(pos: [i32; 3], offset: [i32; 3]) -> [i32; 3] {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

// ---------------------------------------------------------------------------
// The retry guard
// ---------------------------------------------------------------------------

/// When an item last went into an altar from a click (for the other-hand retry guard).
fn recent_inserts() -> &'static Mutex<Vec<(AltarKey, Instant)>> {
    static INSERTS: OnceLock<Mutex<Vec<(AltarKey, Instant)>>> = OnceLock::new();
    INSERTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn mark_insert(key: AltarKey) {
    let mut inserts = recent_inserts()
        .lock()
        .expect("the insert guard is only locked for one push or read");
    inserts.push((key, Instant::now()));
}

fn inserted_recently(key: &AltarKey) -> bool {
    let mut inserts = recent_inserts()
        .lock()
        .expect("the insert guard is only locked for one push or read");
    inserts.retain(|(_, when)| when.elapsed() < RETRY_GUARD);
    inserts.iter().any(|(other, _)| other == key)
}

// ---------------------------------------------------------------------------
// Saved-compound reads
// ---------------------------------------------------------------------------

/// The altar's saved state, as the block entity's compound holds it.
fn read_altar(nbt: &Compound) -> Altar {
    Altar {
        tier: read_int(nbt, KEY_TIER),
        capacity_mod: read_float(nbt, KEY_CAP).max(0.0),
        lp: read_int(nbt, KEY_LP),
        reserve: read_int(nbt, KEY_RESERVE),
        progress: read_int(nbt, KEY_PROGRESS),
        cooldown: read_int(nbt, KEY_COOLDOWN),
        recipe: recipe_id(nbt),
    }
}

/// Writes the altar's state back into the block entity's compound.
fn write_altar(nbt: &mut Compound, altar: &Altar, check: i32) {
    write_int(nbt, KEY_TIER, altar.tier);
    write_float(nbt, KEY_CAP, altar.capacity_mod);
    write_int(nbt, KEY_LP, altar.lp);
    write_int(nbt, KEY_RESERVE, altar.reserve);
    write_int(nbt, KEY_PROGRESS, altar.progress);
    write_int(nbt, KEY_COOLDOWN, altar.cooldown);
    match altar.recipe {
        Some(id) => {
            nbt.insert(KEY_RECIPE.into(), Tag::String(id.into()));
        }
        None => {
            nbt.remove(&KEY_RECIPE.into());
        }
    }
    write_int(nbt, KEY_CHECK, check);
}

/// The recipe in progress, matched back to the table (`Altar.recipe` is a borrowed id upstream
/// too: a renamed recipe just resets the progress).
fn recipe_id(nbt: &Compound) -> Option<&'static str> {
    let Tag::String(id) = nbt.get(&KEY_RECIPE.into())? else {
        return None;
    };
    let id = id.to_string_lossy();
    runtime::RECIPES
        .iter()
        .find(|recipe| recipe.id == id)
        .map(|recipe| recipe.id)
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn read_float(nbt: &Compound, key: &str) -> f32 {
    match nbt.get(&key.into()) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 1.0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn write_float(nbt: &mut Compound, key: &str, value: f32) {
    nbt.insert(key.into(), Tag::Float(value.to_bits()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_altar_declares_one_slot_on_every_face_and_no_menu() {
        let container = container();
        assert_eq!(container.slots, 1);
        assert_eq!(
            container.faces,
            std::array::from_fn::<_, 6, _>(|_| vec![SLOT])
        );
        assert_eq!(container.inputs, vec![SLOT]);
        assert_eq!(container.outputs, vec![SLOT]);
        assert_eq!(container.menu, None);
    }

    #[test]
    fn the_commands_are_the_three_this_port_documents() {
        let mut commands = Vec::new();
        declare_commands(&mut commands);
        let usages: Vec<(&str, u8)> = commands
            .iter()
            .map(|command| (command.usage.as_str(), command.level))
            .collect();
        assert_eq!(
            usages,
            vec![
                ("neovitae altar <pos:block_pos>", 0),
                ("neovitae lp <pos:block_pos> <amount:integer>", 2),
                ("neovitae insert <pos:block_pos> <item:word>", 2),
            ]
        );
    }

    #[test]
    fn an_altar_round_trips_through_its_saved_compound() {
        let altar = Altar {
            tier: 1,
            capacity_mod: 1.8,
            lp: 12_345,
            reserve: 4_000,
            progress: 200,
            cooldown: 17,
            recipe: Some("weak_blood_orb"),
        };
        let mut nbt = Compound::new();
        write_altar(&mut nbt, &altar, 42);
        assert_eq!(read_altar(&nbt), altar);
        assert_eq!(read_int(&nbt, KEY_CHECK), 42);
        assert_eq!(read_int(&nbt, KEY_TIER), 1);
    }

    #[test]
    fn a_finished_recipe_leaves_no_recipe_key_behind() {
        let mut nbt = Compound::new();
        write_altar(&mut nbt, &Altar::new(), 0);
        assert!(!nbt.contains_key(&KEY_RECIPE.into()));
        assert_eq!(read_altar(&nbt).recipe, None);
    }

    #[test]
    fn a_fresh_compound_reads_as_the_weak_altar() {
        // A declared block entity starts with an empty compound: `tier` reads 0 (weak, the value
        // the first structure check confirms), and the absent capacity multiplier defaults to 1.
        let fresh = read_altar(&Compound::new());
        assert_eq!(fresh.tier, 0);
        assert_eq!(fresh.capacity(), 10_000);
        assert_eq!(fresh.lp, 0);
        assert_eq!(fresh.recipe, None);
    }

    #[test]
    fn a_recipe_id_that_is_not_in_the_table_is_dropped() {
        let mut nbt = Compound::new();
        nbt.insert(KEY_RECIPE.into(), Tag::String("no_such_recipe".into()));
        assert_eq!(read_altar(&nbt).recipe, None);
    }

    #[test]
    fn a_click_at_the_altar_is_keyed_by_its_dimension_and_position() {
        // The two altars at one position in two dimensions are two keys, as 0.1.0's repair asked.
        let overworld = AltarKey::new("minecraft:overworld", [1, 2, 3]);
        let end = AltarKey::new("minecraft:the_end", [1, 2, 3]);
        assert_ne!(overworld, end);
    }
}
