//! Machine upgrades, the energy slot and redstone control (M05/M07), Horizon-free.
//!
//! Upstream: `mekanism.common.util.UpgradeUtils` (the formulas), `mekanism.api.upgrade.Upgrade` /
//! `MekanismDatapackRegistries` (the maximum of each kind), `TileEntityMekanism` (muffling volume,
//! redstone control) and `IRedstoneControl.RedstoneControl`. Upstream 26.3 has one `mekanism:upgrade` item
//! with a data component; here each kind is its own item (`mekanism:upgrade_speed`...), since Horizon items
//! carry no such typed component.

/// `general.maxUpgradeMultiplier` (`GeneralConfig`), the default.
pub const MULTIPLIER: f64 = 10.0;

/// Upgrade kinds whose effect is implemented. The others exist as items only (see `ITEM_ONLY`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upgrade {
    Speed,
    Energy,
    Muffling,
}

/// All kinds in the order the machines store them.
pub const KINDS: [Upgrade; 3] = [Upgrade::Speed, Upgrade::Energy, Upgrade::Muffling];

/// Items for the upstream upgrades whose effect needs systems that do not exist here yet (chemical:
/// the gas use of some machines is not modelled; anchor: chunk loading; filter: Electric Pump; stone
/// generator: Cobblestone Generator; XP containment: Xp). A machine does not take them.
pub const ITEM_ONLY: [&str; 5] = ["chemical", "anchor", "filter", "stone_generator", "xp_containment"];

impl Upgrade {
    /// The item's id (`mekanism:upgrade_<name>`).
    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Speed => "speed",
            Upgrade::Energy => "energy",
            Upgrade::Muffling => "muffling",
        }
    }

    /// `Upgrade.max()` from `MekanismDatapackRegistries` (26.3): speed and energy 8, muffling 1
    /// (`Upgrade.create` without a maximum is 1).
    pub fn max(self) -> u32 {
        match self {
            Upgrade::Speed | Upgrade::Energy => 8,
            Upgrade::Muffling => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Upgrade::Speed => "Speed",
            Upgrade::Energy => "Energy",
            Upgrade::Muffling => "Muffling",
        }
    }

    pub fn item(self) -> &'static str {
        match self {
            Upgrade::Speed => "mekanism:upgrade_speed",
            Upgrade::Energy => "mekanism:upgrade_energy",
            Upgrade::Muffling => "mekanism:upgrade_muffling",
        }
    }

    pub fn from_index(i: usize) -> Upgrade {
        KINDS[i]
    }

    pub fn from_item(item: &str) -> Option<Upgrade> {
        KINDS.into_iter().find(|u| u.item() == item)
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Installed upgrades of one machine, by `Upgrade::index`.
pub type Installed = [u32; 3];

/// `UpgradeUtils.fractionUpgrades`: installed over the maximum.
fn fraction(installed: u32, kind: Upgrade) -> f64 {
    installed as f64 / kind.max() as f64
}

/// `UpgradeUtils.getTicks`: the operating ticks with `speed` upgrades; never below 1.
pub fn ticks(def: u32, speed: u32) -> u32 {
    let d = def as f64 * MULTIPLIER.powf(-fraction(speed, Upgrade::Speed));
    // `MathUtils.clampToInt` truncates.
    (d.min(i32::MAX as f64) as u32).max(1)
}

/// `UpgradeUtils.getEnergyPerTick`: `ceil(def * 10^(2 * speed/8 - energy/8))`.
pub fn energy_per_tick(def: u64, speed: u32, energy: u32) -> u64 {
    let exp = 2.0 * fraction(speed, Upgrade::Speed) - fraction(energy, Upgrade::Energy);
    (def as f64 * MULTIPLIER.powf(exp)).ceil() as u64
}

/// `UpgradeUtils.getMaxEnergy`: `def * 10^(energy/8)`, truncated.
pub fn max_energy(def: u64, energy: u32) -> u64 {
    (def as f64 * MULTIPLIER.powf(fraction(energy, Upgrade::Energy))) as u64
}

/// `TileEntityMekanism.getSoundVolume`: the looped sound's volume, `1 - muffling / max`.
pub fn volume(muffling: u32) -> f32 {
    1.0 - muffling.min(Upgrade::Muffling.max()) as f32 / Upgrade::Muffling.max() as f32
}

/// How many of `kind` can still be installed, given the stack's size.
pub fn room(installed: &Installed, kind: Upgrade) -> u32 {
    kind.max().saturating_sub(installed[kind.index()])
}

/// `RedstoneControl` (`IRedstoneControl`): the order is the button's cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redstone {
    Disabled,
    High,
    Low,
    Pulse,
}

impl Redstone {
    pub fn from_index(i: i32) -> Redstone {
        match i {
            1 => Redstone::High,
            2 => Redstone::Low,
            3 => Redstone::Pulse,
            _ => Redstone::Disabled,
        }
    }

    pub fn index(self) -> i32 {
        self as i32
    }

    /// `IIncrementalEnum.getNext`: the mode after a click.
    pub fn next(self) -> Redstone {
        Redstone::from_index((self.index() + 1) % 4)
    }

    pub fn label(self) -> &'static str {
        match self {
            Redstone::Disabled => "Disabled",
            Redstone::High => "High",
            Redstone::Low => "Low",
            Redstone::Pulse => "Pulse",
        }
    }

    /// `TileEntityMekanism.isRedstoneActivated`; `last` is whether it was powered the tick before.
    pub fn active(self, powered: bool, last: bool) -> bool {
        match self {
            Redstone::Disabled => true,
            Redstone::High => powered,
            Redstone::Low => !powered,
            Redstone::Pulse => powered && !last,
        }
    }
}

/// The energy slot (`EnergyInventorySlot.fillOrConvert`): what one tick moves from the item into the machine,
/// at most the item's charge rate, what it holds and what the machine has room for.
pub fn charge(machine: i32, capacity: i32, item: i32, rate: i32) -> i32 {
    rate.min(item).min(capacity - machine).max(0)
}

/// Muffling is a machine-side matter only when it has a sound, and anything is only installed in
/// the machines of `machine::MACHINES` (all of which support speed, energy and muffling upstream).
/// What an upgrade slot may take: an upgrade item of a kind with room.
pub fn accepts(installed: &Installed, item: &str) -> bool {
    Upgrade::from_item(item).is_some_and(|u| room(installed, u) > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximums_match_upstream() {
        assert_eq!((Upgrade::Speed.max(), Upgrade::Energy.max(), Upgrade::Muffling.max()), (8, 8, 1));
    }

    #[test]
    fn ticks_follow_the_exponent() {
        assert_eq!(ticks(200, 0), 200);
        // 200 / 10^(1/8) = 149.98, truncated like MathUtils.clampToInt
        assert_eq!(ticks(200, 1), 149);
        // 200 / 10 at the maximum
        assert_eq!(ticks(200, 8), 20);
        assert_eq!(ticks(5, 8), 1, "never below one tick");
    }

    #[test]
    fn energy_use_is_squared_by_speed_and_eased_by_energy() {
        assert_eq!(energy_per_tick(50, 0, 0), 50);
        // speed maxed: 50 * 10^2
        assert_eq!(energy_per_tick(50, 8, 0), 5000);
        // energy maxed: 50 / 10
        assert_eq!(energy_per_tick(50, 0, 8), 5);
        // both maxed: 50 * 10^(2-1), the energy upgrade only takes the edge off.
        assert_eq!(energy_per_tick(50, 8, 8), 500);
        // ceil: 50 / 10^(1/8) = 37.5
        assert_eq!(energy_per_tick(50, 0, 1), 38);
    }

    #[test]
    fn capacity_grows_with_energy_upgrades() {
        assert_eq!(max_energy(20_000, 0), 20_000);
        assert_eq!(max_energy(20_000, 8), 200_000);
    }

    #[test]
    fn muffling_lowers_the_volume() {
        assert_eq!(volume(0), 1.0);
        // One muffling upgrade is the maximum, and silences the machine (`getSoundVolume`).
        assert_eq!(volume(1), 0.0);
        assert_eq!(volume(9), 0.0);
    }

    #[test]
    fn upgrade_slot_takes_only_upgrades_with_room() {
        let mut installed: Installed = [0; 3];
        assert!(accepts(&installed, "mekanism:upgrade_speed"));
        assert!(!accepts(&installed, "mekanism:upgrade_chemical"));
        assert!(!accepts(&installed, "minecraft:stone"));
        installed[Upgrade::Muffling.index()] = 1;
        assert!(!accepts(&installed, "mekanism:upgrade_muffling"));
        assert_eq!(room(&installed, Upgrade::Speed), 8);
    }

    #[test]
    fn energy_slot_moves_at_most_rate_charge_and_room() {
        assert_eq!(charge(0, 100, 1_000, 40), 40, "rate caps it");
        assert_eq!(charge(0, 100, 10, 40), 10, "the item's charge caps it");
        assert_eq!(charge(90, 100, 1_000, 40), 10, "the machine's room caps it");
        assert_eq!(charge(100, 100, 1_000, 40), 0);
        assert_eq!(charge(150, 100, 1_000, 40), 0, "over capacity takes nothing");
    }

    #[test]
    fn redstone_modes() {
        assert!(Redstone::Disabled.active(false, false) && Redstone::Disabled.active(true, true));
        assert!(Redstone::High.active(true, true) && !Redstone::High.active(false, false));
        assert!(Redstone::Low.active(false, false) && !Redstone::Low.active(true, false));
        // Pulse: only on the tick the signal rises.
        assert!(Redstone::Pulse.active(true, false));
        assert!(!Redstone::Pulse.active(true, true));
        assert!(!Redstone::Pulse.active(false, false));
    }

    #[test]
    fn redstone_button_cycles_through_all_modes() {
        let mut m = Redstone::Disabled;
        let mut seen = vec![];
        for _ in 0..4 {
            seen.push(m);
            m = m.next();
        }
        assert_eq!(seen, [Redstone::Disabled, Redstone::High, Redstone::Low, Redstone::Pulse]);
        assert_eq!(m, Redstone::Disabled);
        assert_eq!(Redstone::from_index(99), Redstone::Disabled);
    }
}
