//! Standalone energy logic for tests. Not yet connected to Horizon block entities.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnergyStorage {
    stored: u64,
    capacity: u64,
}

impl EnergyStorage {
    pub fn new(capacity: u64) -> Self {
        Self { stored: 0, capacity }
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
}
