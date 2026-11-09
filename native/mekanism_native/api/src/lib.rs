//! Mekanism energy cubes (all tiers) and the Energy Tablet for Horizon.
//! Blocks, items and recipes are registered by the separate content mod.
//!
//! Hooks used (native hooks API; see Horizon docs/native-mods.md):
//! - `block_entity` (API 15): a saved block entity for each cube.
//! - `block_entity_energy` (API 142): the cube's energy buffer. Horizon saves the
//!   amount itself in the block entity NBT as `energy` (Tag::Int).
//! - `block_entity_ticks` / `block_entity_tick` (API 15/17): keeps the creative
//!   cube full.
//! - `item_energy_buffer` (API 307): the Energy Tablet's buffer, saved by Horizon
//!   in the stack's `minecraft:custom_data` as `horizon_energy`.
//!
//! Horizon's energy API uses i32 amounts. Mekanism's tiers up to Ultimate fit;
//! the creative cube (Long.MAX_VALUE in Mekanism) is capped at i32::MAX.

use horizon_client::mod_api::{
    BlockEntityEnergyEvent, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent, Compound,
    Hooks, Host, ItemEnergyBuffer, ItemEnergyBufferEvent, ModEnergyBuffer, Tag,
};

/// (block id, capacity, max insert/extract per call) from Mekanism's `EnergyCubeTier`.
const CUBES: [(&str, i32, i32); 5] = [
    ("mekanism:basic_energy_cube", 4_000_000, 4_000),
    ("mekanism:advanced_energy_cube", 16_000_000, 16_000),
    ("mekanism:elite_energy_cube", 64_000_000, 64_000),
    ("mekanism:ultimate_energy_cube", 256_000_000, 256_000),
    ("mekanism:creative_energy_cube", i32::MAX, i32::MAX),
];
const CREATIVE_CUBE: &str = "mekanism:creative_energy_cube";
/// Mekanism `GearConfig`: tablet max energy 1,000,000, charge rate 5,000.
const TABLET: &str = "mekanism:energy_tablet";
const TABLET_CAPACITY: i32 = 1_000_000;
const TABLET_RATE: i32 = 5_000;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(MekanismNative)
}

fn cube(block: &str) -> Option<(i32, i32)> {
    CUBES
        .iter()
        .find(|(id, ..)| *id == block)
        .map(|&(_, capacity, rate)| (capacity, rate))
}

/// The `energy` key in the compound's own key type (Horizon's NBT string).
fn energy_key<K: From<&'static str>>(_: &std::collections::BTreeMap<K, Tag>) -> K {
    "energy".into()
}

struct MekanismNative;

impl Hooks for MekanismNative {
    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent) -> Option<(String, Compound)> {
        cube(event.block).map(|_| (event.block.to_owned(), Compound::new()))
    }

    fn block_entity_energy(&self, _host: &dyn Host, event: &mut BlockEntityEnergyEvent) -> Option<ModEnergyBuffer> {
        cube(event.block).map(|(capacity, rate)| ModEnergyBuffer {
            capacity,
            max_insert: rate,
            max_extract: rate,
            insert_faces: [true; 6],
            extract_faces: [true; 6],
        })
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent) {
        if event.block == CREATIVE_CUBE {
            event.ticks = true;
        }
    }

    /// The creative cube never runs dry: refill it every tick.
    fn block_entity_tick(&self, _host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        if event.entity.id != CREATIVE_CUBE {
            return;
        }
        let full = Tag::Int(i32::MAX);
        let key = energy_key(&event.entity.nbt);
        // Read through `Deref` first: a write marks the entity changed (and resyncs it).
        if event.entity.nbt.get(&key) != Some(&full) {
            event.entity.nbt.insert(key, full);
        }
    }

    fn item_energy_buffer(&self, _host: &dyn Host, event: &mut ItemEnergyBufferEvent) -> Option<ItemEnergyBuffer> {
        (event.item == TABLET).then_some(ItemEnergyBuffer {
            capacity: TABLET_CAPACITY,
            max_insert: TABLET_RATE,
            max_extract: TABLET_RATE,
        })
    }
}
