//! Neo Vitae for Horizon — the Ara Vitae altar, the Capacity Rune family and the Novicius Orb.
//!
//! Ported from Neo Vitae 1.1.51 (`breakinblocks/NeoVitae`, MIT). [`runtime`] is the pure truth — the
//! altar's tiers and layouts, the tank and rune arithmetic, the recipe table and the crafting loop,
//! with its tests — and is byte-identical to the retired `.hnmod` port's file. [`support`] is the
//! glue: the altar's block entity, its slot, the two clicks, the tick, the orb and the commands.
//!
//! # Where the retired container's hooks went
//!
//! The 2026-11-17 mod-API merge (#16941, launcher ≥ 0.2.70) retired `.hnmod`: the plan, the
//! injection points and the per-hook helper files are gone. A hook is a typed method here, and a
//! moment that belongs to one block or one item is a **Part** attached to it.
//!
//! | `.hnmod` hook (API) | here |
//! |---|---|
//! | the `content_root` injection (AE2 pattern) | gone: one flat `.hmod` carries `assets/` and `data/` itself |
//! | `block_entity` (15) | the block entity **type** declared in [`Mod::register`] |
//! | `block_entity_ticks` (15) | `support::AltarBlock` (`BlockEntityPart::block_entity_ticks`) |
//! | `block_entity_container` (16/17) | `Registrar::block_container` in [`Mod::register`] |
//! | `block_entity_tick` (17) | `BlockEntityPart::block_entity_tick` on `support::AltarBlock` |
//! | `use_item_on` (13) | `BlockPart::use_item_on` on `support::AltarBlock` |
//! | `use_item` (13) | `ItemPart::use_item` on `support::Orb` |
//! | `right_click_block` (21) | `BlockPart::use_without_item` on `support::AltarBlock` |
//! | `register_commands` (8) | [`Mod::register_commands`] |
//! | `perform_command` (8) | [`Mod::run_mod_command`] (typed arguments, not a split line) |
pub mod runtime;
pub mod support;

use mod_api::{
    self, Flow, Hooks, RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent,
    ServerHost,
};

/// The mod keeps no state of its own: the altar's lives in its block entity's saved compound, and
/// the queued clicks live in `support`'s queues.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare what the game answers for the altar (`EntityBlock` plus NeoForge's
    /// `RegisterCapabilitiesEvent`): its block entity type, its one-slot container, the block Part
    /// that takes both clicks and ticks, and the orb's item Part.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        support::declare(r);
    }

    /// `/neovitae altar|lp|insert`.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::declare_commands(event.commands);
    }

    /// Run one of the commands above and take it.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::run_command(host, event)
    }
}
