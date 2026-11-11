//! Standalone energy logic for tests. Not yet connected to Horizon block entities.
//!
//! `cube` is the native mod's own side-config and transfer module, included by
//! path so its tests run here without building Horizon.

#[path = "../../../native/mekanism_native/api/src/cube.rs"]
pub mod cube;

/// The Metallurgic Infuser's Horizon-free part (buffer, operation, menu data).
#[path = "../../../native/mekanism_native/api/src/infuser.rs"]
pub mod infuser;

/// The electric machines' table and operation.
#[path = "../../../native/mekanism_native/api/src/machine.rs"]
pub mod machine;

/// What takes energy from a neighbouring Mekanism block, and how much.
#[path = "../../../native/mekanism_native/api/src/energy.rs"]
pub mod energy;

/// Heat and Solar Generator logic (M04).
#[path = "../../../native/mekanism_native/api/src/generator.rs"]
pub mod generator;

/// Fluids, chemicals and the tanks (M06).
#[path = "../../../native/mekanism_native/api/src/chemicals.rs"]
pub mod chemicals;
#[path = "../../../native/mekanism_native/api/src/tank.rs"]
pub mod tank;

/// Canteen, Gauge Dropper and Atomic Disassembler rules (M11).
#[path = "../../../native/mekanism_native/api/src/gear.rs"]
pub mod gear;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnergyStorage {
    stored: u64,
    capacity: u64,
}

impl EnergyStorage {
    pub fn new(capacity: u64) -> Self {
        Self { stored: 0, capacity }
    }

    /// Create from a saved amount (invalid/old larger values are clamped).
    pub fn from_saved(capacity: u64, stored: u64) -> Self {
        Self { stored: stored.min(capacity), capacity }
    }

    pub fn stored(&self) -> u64 { self.stored }
    pub fn capacity(&self) -> u64 { self.capacity }

    /// Returns the amount actually accepted, without overflow.
    pub fn receive(&mut self, amount: u64) -> u64 {
        let accepted = amount.min(self.capacity - self.stored);
        self.stored += accepted;
        accepted
    }

    /// Returns the amount actually extracted.
    pub fn extract(&mut self, amount: u64) -> u64 {
        let extracted = amount.min(self.stored);
        self.stored -= extracted;
        extracted
    }

    pub fn set_capacity(&mut self, capacity: u64) {
        self.capacity = capacity;
        self.stored = self.stored.min(capacity);
    }
}

// Horizon's block capability works with i32 energy; match the Java defaults.
pub const BASIC_CUBE_CAPACITY: u64 = 4_000_000;
pub const BASIC_CUBE_TRANSFER_PER_CALL: u64 = 4_000;

/// Test-only reference model. The native Horizon hook manages live NBT itself.
pub struct BasicEnergyCube {
    energy: EnergyStorage,
}

impl Default for BasicEnergyCube {
    fn default() -> Self { Self::new() }
}

impl BasicEnergyCube {
    pub fn new() -> Self {
        Self { energy: EnergyStorage::new(BASIC_CUBE_CAPACITY) }
    }

    pub fn from_saved(stored: u64) -> Self {
        Self { energy: EnergyStorage::from_saved(BASIC_CUBE_CAPACITY, stored) }
    }

    pub fn stored(&self) -> u64 { self.energy.stored() }
    pub fn capacity(&self) -> u64 { self.energy.capacity() }

    pub fn insert(&mut self, offered: u64) -> u64 {
        self.energy.receive(offered.min(BASIC_CUBE_TRANSFER_PER_CALL))
    }

    pub fn extract(&mut self, requested: u64) -> u64 {
        self.energy.extract(requested.min(BASIC_CUBE_TRANSFER_PER_CALL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_up_to_capacity() {
        let mut e = EnergyStorage::new(100);
        assert_eq!(e.receive(60), 60);
        assert_eq!(e.receive(70), 40);
        assert_eq!(e.stored(), 100);
    }

    #[test]
    fn cannot_extract_more_than_stored() {
        let mut e = EnergyStorage::new(100);
        assert_eq!(e.receive(30), 30);
        assert_eq!(e.extract(50), 30);
        assert_eq!(e.stored(), 0);
    }

    #[test]
    fn capacity_decrease_clamps_storage() {
        let mut e = EnergyStorage::new(200);
        e.receive(150);
        e.set_capacity(75);
        assert_eq!(e.stored(), 75);
        assert_eq!(e.capacity(), 75);
    }

    #[test]
    fn handles_maximum_values_without_overflow() {
        let mut e = EnergyStorage::new(u64::MAX);
        assert_eq!(e.receive(u64::MAX), u64::MAX);
        assert_eq!(e.receive(1), 0);
        assert_eq!(e.extract(u64::MAX), u64::MAX);
    }
    #[test]
    fn basic_cube_uses_mekanism_default_limits() {
        let mut cube = BasicEnergyCube::new();
        assert_eq!(cube.capacity(), 4_000_000);
        assert_eq!(cube.insert(100_000), 4_000);
        assert_eq!(cube.stored(), 4_000);
        assert_eq!(cube.extract(100_000), 4_000);
        assert_eq!(cube.stored(), 0);
    }

    #[test]
    fn basic_cube_restores_saved_energy() {
        let cube = BasicEnergyCube::from_saved(3_250_000);
        assert_eq!(cube.stored(), 3_250_000);
        let mut reloaded = BasicEnergyCube::from_saved(cube.stored());
        assert_eq!(reloaded.extract(9_000), 4_000);
    }

    #[test]
    fn basic_cube_clamps_corrupt_saved_amount() {
        let cube = BasicEnergyCube::from_saved(u64::MAX);
        assert_eq!(cube.stored(), BASIC_CUBE_CAPACITY);
    }

}
