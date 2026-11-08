//! Basic Energy Cube using Horizon native hook API 15 + API 142.
//! The block and its recipe are registered by the separate content mod.
//! Its saved energy is managed by Horizon, in the block entity NBT `energy` (Tag::Int).
//!
//! Test with a Horizon build whose native hook API is at least 142.
//! This native companion must be built with `horizon-launcher native-build`.

use horizon_client::mod_api::{
    BlockEntityEnergyEvent, BlockEntityEvent, Compound, Hooks, Host, ModEnergyBuffer,
};

const CUBE: &str = "mekanism:basic_energy_cube";
// Values from Mekanism's EnergyCubeTier.BASIC defaults (in Joules).
// Horizon's energy hook uses i32; these values fit without truncation.
const CAPACITY: i32 = 4_000_000;
const TRANSFER_PER_CALL: i32 = 4_000;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(MekanismNative)
}

struct MekanismNative;

impl Hooks for MekanismNative {
    /// API 15: create an actual saved block entity for our custom content block.
    fn block_entity(
        &self,
        _host: &mut dyn Host,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<(String, Compound)> {
        (event.block == CUBE).then(|| (CUBE.to_owned(), Compound::new()))
    }

    /// API 142: expose the block's persisted energy capability on all six faces.
    /// The amount itself is saved/loaded automatically by Horizon as NBT `energy`.
    fn block_entity_energy(
        &self,
        _host: &dyn Host,
        event: &mut BlockEntityEnergyEvent<'_>,
    ) -> Option<ModEnergyBuffer> {
        (event.block == CUBE).then_some(ModEnergyBuffer {
            capacity: CAPACITY,
            max_insert: TRANSFER_PER_CALL,
            max_extract: TRANSFER_PER_CALL,
            insert_faces: [true; 6],
            extract_faces: [true; 6],
        })
    }
}
