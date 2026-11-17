//! Frequencies (M08), the parts that need no Horizon types, so their tests run in
//! `logic/energy`. The host sides are `teleporters.rs` and `entangloporter.rs`.
//!
//! Mekanism 26.3 (`lib/frequency/Frequency`, `FrequencyController`, `FrequencyType`,
//! `content/teleporter/TeleporterFrequency`, `content/entangloporter/InventoryFrequency`):
//! a frequency is a name plus a security mode (public or private) plus the owner's UUID.
//! A public name exists once for everyone; a private name exists once per owner and only
//! that owner may bind to it. A teleporter frequency keeps the set of its active teleporters
//! and picks the closest one (`TeleporterFrequency.getClosestCoords`); an inventory
//! frequency (the Quantum Entangloporter's) keeps the shared energy, fluid, chemical and
//! item buffers (heat is not ported).
//!
//! Differences from Mekanism, all because Horizon's mod screens have no text entry:
//! names are fixed numbered channels ("1" to `CHANNELS`) in the GUI; the registry itself
//! takes any name. There is no frequency colour, no trusted list and no Security Desk.
//!
//! The registry is saved by the host in the world folder (`save`/`load`); which teleporters
//! are active is never saved, it is rebuilt by their ticks (`touch`), as Mekanism's active
//! coords are.

use std::collections::BTreeMap;

pub type Pos = [i32; 3];

/// How many numbered channels the GUI offers per security mode.
pub const CHANNELS: u8 = 16;

/// Mekanism's `GeneralConfig` quantum_entangloporter buffers: the Ultimate energy cube,
/// fluid tank and chemical tank capacities (`EnergyCubeTier`, `FluidTankTier`,
/// `ChemicalTankTier`). The energy is 256,000,000 FE; Horizon's block-entity energy is an
/// `i32`, so the members' own buffers are only the windows below, the pool is 64-bit here.
pub const ENERGY_CAP: i64 = 256_000_000;
pub const FLUID_CAP: i64 = 256_000;
pub const CHEMICAL_CAP: i64 = 8_192_000;
/// What one member exposes to its neighbours: the Ultimate tiers' per-tick rates
/// (`EnergyCubeTier` 256,000, `FluidTankTier` 64,000, `ChemicalTankTier` 512,000).
pub const ENERGY_WINDOW: i64 = 256_000;
pub const FLUID_WINDOW: i64 = 64_000;
pub const CHEMICAL_WINDOW: i64 = 512_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Teleporter,
    Inventory,
}

impl Kind {
    fn tag(self) -> &'static str {
        match self {
            Kind::Teleporter => "teleporter",
            Kind::Inventory => "inventory",
        }
    }
    fn from_tag(s: &str) -> Option<Kind> {
        match s {
            "teleporter" => Some(Kind::Teleporter),
            "inventory" => Some(Kind::Inventory),
            _ => None,
        }
    }
}

/// What a block or item remembers to find its frequency again (`FrequencyIdentity`).
/// `owner` is the creator for a public frequency (it does not take part in the lookup)
/// and the only user for a private one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub kind: Kind,
    pub name: String,
    pub public: bool,
    pub owner: u128,
}

impl Identity {
    pub fn new(kind: Kind, name: &str, public: bool, owner: u128) -> Identity {
        Identity { kind, name: name.to_owned(), public, owner }
    }

    /// The lookup key: a public name ignores the owner.
    fn key(&self) -> (Kind, String, bool, u128) {
        (self.kind, self.name.clone(), self.public, if self.public { 0 } else { self.owner })
    }
}

/// The name of GUI channel `channel` (1-based).
pub fn channel_name(channel: u8) -> String {
    channel.to_string()
}

/// The channel a name stands for, if it is one of the GUI's.
pub fn channel_of(name: &str) -> Option<u8> {
    name.parse::<u8>().ok().filter(|c| (1..=CHANNELS).contains(c))
}

/// A teleporter or entangloporter in the world.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Member {
    pub dimension: String,
    pub pos: Pos,
}

impl Member {
    pub fn new(dimension: &str, pos: Pos) -> Member {
        Member { dimension: dimension.to_owned(), pos }
    }
}

/// One buffer's content: what it holds (`None` for energy) and how much.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tank {
    pub kind: Option<String>,
    pub amount: i64,
}

impl Tank {
    pub fn new(kind: Option<&str>, amount: i64) -> Tank {
        Tank { kind: kind.map(str::to_owned), amount }
    }

    fn tidy(&mut self) {
        if self.amount <= 0 {
            self.amount = 0;
            self.kind = None;
        }
    }
}

/// An inventory frequency's shared buffers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shared {
    pub energy: i64,
    pub fluid: Tank,
    pub chemical: Tank,
    /// One slot: the item's id and its count.
    pub item: Tank,
}

/// Moves what a member holds locally into the shared `pool`, then hands the member
/// a window of the pool back. Every tick, per member:
///
/// 1. the member's whole local content goes into the pool (as far as the pool has room
///    and holds the same thing; what does not fit, or is another fluid, stays local);
/// 2. the member's local buffer is refilled from the pool up to `window`.
///
/// Nothing is made or lost: `pool + local` is the same before and after, for any
/// number of members in the same tick. A member loaded with a stale local value just
/// gives it to the pool. `cap` is the pool's capacity.
pub fn exchange(pool: &mut Tank, cap: i64, local: &mut Tank, window: i64) {
    local.tidy();
    if local.amount > 0 {
        if pool.amount == 0 {
            pool.kind = local.kind.clone();
        }
        if pool.kind == local.kind {
            let take = local.amount.min(cap - pool.amount).max(0);
            pool.amount += take;
            local.amount -= take;
        }
    }
    local.tidy();
    pool.tidy();
    if pool.amount > 0 && (local.amount == 0 || local.kind == pool.kind) {
        let give = pool.amount.min(window - local.amount).max(0);
        local.kind = pool.kind.clone();
        local.amount += give;
        pool.amount -= give;
    }
    pool.tidy();
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frequency {
    pub identity: Identity,
    pub owner_name: String,
    pub shared: Shared,
    /// Active members and when each was last seen (milliseconds, the caller's clock).
    active: BTreeMap<Member, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A public name is taken, or a private one by the same owner.
    Exists,
    /// There is no such frequency.
    Missing,
    /// The player owns neither the frequency nor an operator's rights.
    NotOwner,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    map: BTreeMap<(Kind, String, bool, u128), Frequency>,
    /// Set by every change that `save` would write.
    pub dirty: bool,
}

/// An operator (permission level 2 and up) may use and delete any frequency.
pub fn is_operator(permission: u8) -> bool {
    permission >= 2
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    pub fn get(&self, id: &Identity) -> Option<&Frequency> {
        self.map.get(&id.key())
    }

    pub fn get_mut(&mut self, id: &Identity) -> Option<&mut Frequency> {
        self.map.get_mut(&id.key())
    }

    /// `FrequencyController.addFrequency`: creates the frequency. The owner of a public
    /// one is the creator only for display; its identity ignores the owner.
    pub fn create(&mut self, id: &Identity, owner_name: &str) -> Result<(), Error> {
        if self.map.contains_key(&id.key()) {
            return Err(Error::Exists);
        }
        let frequency = Frequency {
            identity: id.clone(),
            owner_name: owner_name.to_owned(),
            shared: Shared::default(),
            active: BTreeMap::new(),
        };
        self.map.insert(id.key(), frequency);
        self.dirty = true;
        Ok(())
    }

    /// Whether `player` may bind a block to the frequency `id`: a public one is open to
    /// everyone, a private one only to its owner.
    pub fn may_bind(id: &Identity, player: u128) -> bool {
        id.public || id.owner == player
    }

    /// `FrequencyController.removeFrequency`: only the creator (or an operator) deletes.
    /// Blocks bound to it find nothing on their next tick and report no frequency.
    pub fn delete(&mut self, id: &Identity, player: u128, permission: u8) -> Result<(), Error> {
        let Some(f) = self.map.get(&id.key()) else { return Err(Error::Missing) };
        if f.identity.owner != player && !is_operator(permission) {
            return Err(Error::NotOwner);
        }
        self.map.remove(&id.key());
        self.dirty = true;
        Ok(())
    }

    /// The frequencies `player` sees in the GUI: every public one and their own.
    pub fn visible(&self, kind: Kind, player: u128) -> Vec<&Frequency> {
        self.map
            .values()
            .filter(|f| f.identity.kind == kind && (f.identity.public || f.identity.owner == player))
            .collect()
    }

    /// `Frequency.update`: the member is active, as of `now`.
    pub fn touch(&mut self, id: &Identity, member: &Member, now: u64) {
        if let Some(f) = self.get_mut(id) {
            f.active.insert(member.clone(), now);
        }
    }

    /// `Frequency.onDeactivate`.
    pub fn deactivate(&mut self, id: &Identity, member: &Member) {
        if let Some(f) = self.get_mut(id) {
            f.active.remove(member);
        }
    }

    /// Removes a member from every frequency (its block was broken or changed).
    pub fn forget(&mut self, member: &Member) {
        for f in self.map.values_mut() {
            f.active.remove(member);
        }
    }

    /// The members seen within `ttl` milliseconds of `now`.
    pub fn active(&self, id: &Identity, now: u64, ttl: u64) -> Vec<Member> {
        self.get(id).map_or_else(Vec::new, |f| {
            f.active.iter().filter(|(_, seen)| now.saturating_sub(**seen) <= ttl).map(|(m, _)| m.clone()).collect()
        })
    }

    /// `TeleporterFrequency.getClosestCoords`, member for member: the active member
    /// other than `from` that is closest by squared block distance, and one in `from`'s own
    /// dimension before any in another (distances across dimensions are raw
    /// coordinates, as Mekanism compares them).
    pub fn closest(&self, id: &Identity, from: &Member, now: u64, ttl: u64) -> Option<Member> {
        let mut closest: Option<Member> = None;
        for iter in self.active(id, now, ttl) {
            if iter == *from {
                continue;
            }
            let Some(c) = &closest else {
                closest = Some(iter);
                continue;
            };
            let dim = &from.dimension;
            if *dim != c.dimension && *dim == iter.dimension {
                closest = Some(iter);
            } else if (*dim != c.dimension || *dim == iter.dimension)
                && dist_sqr(from.pos, c.pos) > dist_sqr(from.pos, iter.pos)
            {
                closest = Some(iter);
            }
        }
        closest
    }

    /// The registry as text, one frequency per line, for the world folder.
    pub fn save(&self) -> String {
        let mut out = String::from("mekanism-frequencies 1\n");
        for f in self.map.values() {
            let s = &f.shared;
            let kind = |t: &Tank| t.kind.clone().unwrap_or_default();
            let fields = [
                f.identity.kind.tag().to_owned(),
                clean(&f.identity.name),
                u8::from(f.identity.public).to_string(),
                format!("{:032x}", f.identity.owner),
                clean(&f.owner_name),
                s.energy.to_string(),
                kind(&s.fluid),
                s.fluid.amount.to_string(),
                kind(&s.chemical),
                s.chemical.amount.to_string(),
                kind(&s.item),
                s.item.amount.to_string(),
            ];
            out.push_str(&fields.join("\t"));
            out.push('\n');
        }
        out
    }

    /// The registry `save` wrote; a line that does not parse is skipped.
    pub fn load(text: &str) -> Registry {
        let mut registry = Registry::new();
        for line in text.lines().skip(1) {
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() != 12 {
                continue;
            }
            let (Some(kind), Ok(owner), Ok(energy)) =
                (Kind::from_tag(p[0]), u128::from_str_radix(p[3], 16), p[5].parse::<i64>())
            else {
                continue;
            };
            let tank = |id: &str, amount: &str| {
                let mut t = Tank { kind: (!id.is_empty()).then(|| id.to_owned()), amount: amount.parse().unwrap_or(0) };
                t.tidy();
                t
            };
            let id = Identity::new(kind, p[1], p[2] == "1", owner);
            let frequency = Frequency {
                identity: id.clone(),
                owner_name: p[4].to_owned(),
                shared: Shared {
                    energy: energy.max(0),
                    fluid: tank(p[6], p[7]),
                    chemical: tank(p[8], p[9]),
                    item: tank(p[10], p[11]),
                },
                active: BTreeMap::new(),
            };
            registry.map.insert(id.key(), frequency);
        }
        registry
    }
}

/// Tabs and line breaks would break the file's columns.
fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// `BlockPos.distSqr`.
pub fn dist_sqr(a: Pos, b: Pos) -> i64 {
    (0..3).map(|i| i64::from(a[i] - b[i])).map(|d| d * d).sum()
}

// --- the frequency panel shared by the Teleporter and the Quantum Entangloporter ---

/// Menu button ids of the frequency panel.
pub const BUTTON_PREV: i32 = 20;
pub const BUTTON_NEXT: i32 = 21;
pub const BUTTON_PUBLIC: i32 = 22;
pub const BUTTON_SET: i32 = 23;
pub const BUTTON_DELETE: i32 = 24;

/// Where the panel's buttons are, `[x, y, w, h]` from the screen origin.
pub const PREV_RECT: [f32; 4] = [8.0, 41.0, 20.0, 14.0];
pub const NEXT_RECT: [f32; 4] = [120.0, 41.0, 20.0, 14.0];
pub const PUBLIC_RECT: [f32; 4] = [8.0, 58.0, 60.0, 14.0];
pub const SET_RECT: [f32; 4] = [72.0, 58.0, 44.0, 14.0];
pub const DELETE_RECT: [f32; 4] = [120.0, 58.0, 48.0, 14.0];

/// The panel button under `at` (a point from the screen origin).
pub fn button_at(at: [f32; 2]) -> Option<i32> {
    let inside = |r: [f32; 4]| at[0] >= r[0] && at[0] < r[0] + r[2] && at[1] >= r[1] && at[1] < r[1] + r[3];
    [
        (PREV_RECT, BUTTON_PREV),
        (NEXT_RECT, BUTTON_NEXT),
        (PUBLIC_RECT, BUTTON_PUBLIC),
        (SET_RECT, BUTTON_SET),
        (DELETE_RECT, BUTTON_DELETE),
    ]
    .into_iter()
    .find(|(r, _)| inside(*r))
    .map(|(_, id)| id)
}

/// The GUI's choice: which channel and mode the panel shows and `Set` binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    /// 1 to `CHANNELS`.
    pub channel: u8,
    pub public: bool,
}

impl Default for Choice {
    fn default() -> Choice {
        Choice { channel: 1, public: true }
    }
}

impl Choice {
    /// `BUTTON_PREV`/`NEXT` step the channel around the ring; `BUTTON_PUBLIC` toggles the mode.
    pub fn press(self, button: i32) -> Choice {
        match button {
            BUTTON_PREV => Choice { channel: if self.channel <= 1 { CHANNELS } else { self.channel - 1 }, ..self },
            BUTTON_NEXT => Choice { channel: if self.channel >= CHANNELS { 1 } else { self.channel + 1 }, ..self },
            BUTTON_PUBLIC => Choice { public: !self.public, ..self },
            _ => self,
        }
    }

    pub fn identity(self, kind: Kind, player: u128) -> Identity {
        Identity::new(kind, &channel_name(self.channel), self.public, player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: u128 = 1;
    const BOB: u128 = 2;

    fn m(dim: &str, x: i32, y: i32, z: i32) -> Member {
        Member::new(dim, [x, y, z])
    }

    #[test]
    fn public_names_are_shared_private_names_are_per_owner() {
        let mut r = Registry::new();
        let public_a = Identity::new(Kind::Teleporter, "1", true, ALICE);
        assert_eq!(r.create(&public_a, "Alice"), Ok(()));
        // Anyone finds the public frequency under the same name, whatever owner they bring.
        let public_b = Identity::new(Kind::Teleporter, "1", true, BOB);
        assert!(r.get(&public_b).is_some());
        assert_eq!(r.create(&public_b, "Bob"), Err(Error::Exists));
        // A private "1" is a different frequency, once per owner.
        let private_a = Identity::new(Kind::Teleporter, "1", false, ALICE);
        let private_b = Identity::new(Kind::Teleporter, "1", false, BOB);
        assert_eq!(r.create(&private_a, "Alice"), Ok(()));
        assert_eq!(r.create(&private_b, "Bob"), Ok(()));
        assert_eq!(r.create(&private_a, "Alice"), Err(Error::Exists));
        assert_eq!(r.get(&private_a).unwrap().owner_name, "Alice");
        assert_eq!(r.get(&private_b).unwrap().owner_name, "Bob");
        // The same name of the other kind is another frequency again.
        assert!(r.get(&Identity::new(Kind::Inventory, "1", true, ALICE)).is_none());
    }

    #[test]
    fn only_the_owner_binds_private_and_deletes() {
        let public = Identity::new(Kind::Teleporter, "3", true, ALICE);
        let private = Identity::new(Kind::Teleporter, "3", false, ALICE);
        assert!(Registry::may_bind(&public, BOB), "public is open to all");
        assert!(Registry::may_bind(&private, ALICE));
        assert!(!Registry::may_bind(&private, BOB), "private is the owner's");

        let mut r = Registry::new();
        r.create(&public, "Alice").unwrap();
        assert_eq!(r.delete(&public, BOB, 0), Err(Error::NotOwner));
        assert_eq!(r.delete(&public, BOB, 1), Err(Error::NotOwner), "level 1 is no operator");
        assert_eq!(r.delete(&public, BOB, 2), Ok(()), "an operator may");
        assert_eq!(r.delete(&public, ALICE, 0), Err(Error::Missing));
        r.create(&public, "Alice").unwrap();
        assert_eq!(r.delete(&public, ALICE, 0), Ok(()), "the creator may");
        assert!(r.get(&public).is_none());
    }

    #[test]
    fn the_gui_lists_public_and_own_frequencies_only() {
        let mut r = Registry::new();
        r.create(&Identity::new(Kind::Teleporter, "1", true, BOB), "Bob").unwrap();
        r.create(&Identity::new(Kind::Teleporter, "2", false, BOB), "Bob").unwrap();
        r.create(&Identity::new(Kind::Teleporter, "3", false, ALICE), "Alice").unwrap();
        r.create(&Identity::new(Kind::Inventory, "4", true, ALICE), "Alice").unwrap();
        let names = |p| r.visible(Kind::Teleporter, p).iter().map(|f| f.identity.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(ALICE), ["1", "3"]);
        assert_eq!(names(BOB), ["1", "2"]);
    }

    #[test]
    fn closest_prefers_the_same_dimension_then_distance() {
        let id = Identity::new(Kind::Teleporter, "1", true, ALICE);
        let mut r = Registry::new();
        r.create(&id, "A").unwrap();
        let here = m("minecraft:overworld", 0, 64, 0);
        let near = m("minecraft:overworld", 10, 64, 0);
        let far = m("minecraft:overworld", -100, 64, 0);
        let nether = m("minecraft:the_nether", 1, 64, 0);
        for member in [&here, &far, &nether, &near] {
            r.touch(&id, member, 1_000);
        }
        // Never itself; the nether one is nearer by raw coordinates but another dimension.
        assert_eq!(r.closest(&id, &here, 1_000, 500), Some(near.clone()));
        // From the nether, the overworld ones are reachable, the nearest by raw coordinates.
        assert_eq!(r.closest(&id, &nether, 1_000, 500), Some(here.clone()));
        // Only another dimension is left: it is taken.
        r.deactivate(&id, &near);
        r.deactivate(&id, &far);
        assert_eq!(r.closest(&id, &here, 1_000, 500), Some(nether.clone()));
        r.deactivate(&id, &nether);
        assert_eq!(r.closest(&id, &here, 1_000, 500), None, "alone: no destination");
    }

    #[test]
    fn members_not_seen_lately_are_not_active() {
        let id = Identity::new(Kind::Teleporter, "1", true, ALICE);
        let mut r = Registry::new();
        r.create(&id, "A").unwrap();
        let a = m("minecraft:overworld", 0, 0, 0);
        let b = m("minecraft:overworld", 5, 0, 0);
        r.touch(&id, &a, 0);
        r.touch(&id, &b, 900);
        assert_eq!(r.active(&id, 1_000, 500), vec![b.clone()], "a was last seen 1000 ms ago");
        assert_eq!(r.closest(&id, &b, 1_000, 500), None);
        r.touch(&id, &a, 1_000);
        assert_eq!(r.closest(&id, &b, 1_000, 500), Some(a.clone()));
        r.forget(&a);
        assert_eq!(r.closest(&id, &b, 1_000, 500), None, "a broken teleporter is gone");
        // Touching a frequency that does not exist does nothing.
        r.touch(&Identity::new(Kind::Teleporter, "9", true, ALICE), &a, 0);
    }

    #[test]
    fn registry_round_trips_through_text() {
        let mut r = Registry::new();
        let id = Identity::new(Kind::Inventory, "7", false, 0x0123_4567_89ab_cdef_0011_2233_4455_6677);
        r.create(&id, "Ali\tce").unwrap();
        r.create(&Identity::new(Kind::Teleporter, "1", true, ALICE), "A").unwrap();
        {
            let f = r.get_mut(&id).unwrap();
            f.shared = Shared {
                energy: 123_456_789_012,
                fluid: Tank::new(Some("minecraft:water"), 5_000),
                chemical: Tank::new(Some("mekanism:chemical/hydrogen"), 64),
                item: Tank::new(Some("minecraft:cobblestone"), 17),
            };
        }
        r.touch(&id, &m("minecraft:overworld", 1, 2, 3), 5);
        let loaded = Registry::load(&r.save());
        let f = loaded.get(&id).unwrap();
        assert_eq!(f.shared, r.get(&id).unwrap().shared);
        assert_eq!(f.owner_name, "Ali ce", "a tab in a name is replaced");
        assert!(f.active.is_empty(), "active members are not saved");
        assert!(loaded.get(&Identity::new(Kind::Teleporter, "1", true, BOB)).is_some());
        assert!(!loaded.dirty);
        // Damaged lines are skipped, not fatal.
        let broken = format!("{}garbage\nteleporter\tx\n", r.save());
        assert_eq!(Registry::load(&broken).visible(Kind::Inventory, 0x0123_4567_89ab_cdef_0011_2233_4455_6677).len(), 1);
    }

    fn total(pool: &Tank, locals: &[Tank]) -> i64 {
        pool.amount + locals.iter().map(|l| l.amount).sum::<i64>()
    }

    #[test]
    fn exchange_gives_a_window_and_takes_the_rest_back() {
        let mut pool = Tank::new(None, 1_000_000);
        let mut local = Tank::default();
        exchange(&mut pool, ENERGY_CAP, &mut local, ENERGY_WINDOW);
        assert_eq!((pool.amount, local.amount), (1_000_000 - 256_000, 256_000));
        // The neighbour took 56,000 and put 6,000 in the cable beside: next tick the rest is reclaimed and a window again.
        local.amount = 200_000 + 6_000;
        exchange(&mut pool, ENERGY_CAP, &mut local, ENERGY_WINDOW);
        assert_eq!(total(&pool, &[local.clone()]), 744_000 + 206_000);
        assert_eq!(local.amount, 256_000);
    }

    #[test]
    fn exchange_over_capacity_keeps_the_excess_local() {
        let mut pool = Tank::new(None, 90);
        let mut local = Tank::new(None, 50);
        exchange(&mut pool, 100, &mut local, 20);
        // 10 fit, 40 stay local; the window is already exceeded so nothing more is given.
        assert_eq!(pool.amount, 100 - 0);
        assert_eq!(local.amount, 40);
        assert_eq!(total(&pool, &[local]), 140);
    }

    #[test]
    fn two_members_in_one_tick_conserve() {
        let mut pool = Tank::new(None, 0);
        let mut a = Tank::new(None, 80);
        let mut b = Tank::new(None, 80);
        exchange(&mut pool, 100, &mut a, 30);
        exchange(&mut pool, 100, &mut b, 30);
        assert_eq!(total(&pool, &[a.clone(), b.clone()]), 160, "nothing minted or lost");
        assert!(pool.amount <= 100);
        // Both then drain their windows completely: the pool never goes negative.
        let before = total(&pool, &[a.clone(), b.clone()]);
        let drained = a.amount + b.amount;
        a.amount = 0;
        b.amount = 0;
        exchange(&mut pool, 100, &mut a, 30);
        exchange(&mut pool, 100, &mut b, 30);
        assert_eq!(total(&pool, &[a, b]), before - drained);
        assert!(pool.amount >= 0);
    }

    #[test]
    fn exchange_never_mixes_fluids() {
        let mut pool = Tank::new(Some("minecraft:water"), 500);
        let mut lava = Tank::new(Some("minecraft:lava"), 100);
        exchange(&mut pool, 1_000, &mut lava, 50);
        assert_eq!(pool, Tank::new(Some("minecraft:water"), 500), "the pool is untouched");
        assert_eq!(lava, Tank::new(Some("minecraft:lava"), 100), "the lava stays where it is");
        // A member holding water and one holding lava in the same tick: both keep what they have, nothing is dropped.
        let mut water = Tank::new(Some("minecraft:water"), 30);
        exchange(&mut pool, 1_000, &mut water, 50);
        assert_eq!(total(&pool, &[water.clone(), lava.clone()]), 630);
        assert_eq!(water.kind.as_deref(), Some("minecraft:water"));
        assert_eq!(lava.kind.as_deref(), Some("minecraft:lava"));
    }

    #[test]
    fn an_empty_pool_adopts_what_arrives_and_forgets_it_when_empty() {
        let mut pool = Tank::default();
        let mut local = Tank::new(Some("minecraft:water"), 10);
        exchange(&mut pool, 100, &mut local, 4);
        assert_eq!(pool.kind.as_deref(), Some("minecraft:water"));
        assert_eq!((pool.amount, local.amount), (6, 4));
        // The member empties its window; later the pool is drained to nothing and forgets the kind.
        local.amount = 0;
        exchange(&mut pool, 100, &mut local, 4);
        local.amount = 0;
        exchange(&mut pool, 100, &mut local, 4);
        assert_eq!(pool.amount, 0);
        assert_eq!(pool.kind, None);
        local = Tank::new(Some("minecraft:lava"), 3);
        exchange(&mut pool, 100, &mut local, 4);
        assert_eq!(local, Tank::new(Some("minecraft:lava"), 3), "a new fluid is accepted once empty");
    }

    #[test]
    fn a_stale_member_gives_its_value_to_the_pool() {
        // A member loads with energy left from a window of an earlier session.
        let mut pool = Tank::new(None, 10);
        let mut stale = Tank::new(None, 256_000);
        exchange(&mut pool, ENERGY_CAP, &mut stale, ENERGY_WINDOW);
        assert_eq!(total(&pool, &[stale.clone()]), 256_010);
        assert_eq!(stale.amount, 256_000, "a window of it is shown again");
    }

    #[test]
    fn the_item_slot_is_shared_by_stack_size() {
        let mut pool = Tank::new(Some("minecraft:cobblestone"), 60);
        let mut slot = Tank::new(Some("minecraft:cobblestone"), 10);
        // Stack size 64: 4 fit, 6 stay in the slot, which then holds 6 of the pool's 64... window 64.
        exchange(&mut pool, 64, &mut slot, 64);
        assert_eq!(total(&pool, &[slot.clone()]), 70);
        assert!(pool.amount <= 64);
        let mut dirt = Tank::new(Some("minecraft:dirt"), 5);
        exchange(&mut pool, 64, &mut dirt, 64);
        assert_eq!(dirt, Tank::new(Some("minecraft:dirt"), 5), "another item is left alone");
    }

    #[test]
    fn panel_buttons_and_choice() {
        assert_eq!(button_at([10.0, 45.0]), Some(BUTTON_PREV));
        assert_eq!(button_at([130.0, 45.0]), Some(BUTTON_NEXT));
        assert_eq!(button_at([20.0, 60.0]), Some(BUTTON_PUBLIC));
        assert_eq!(button_at([80.0, 60.0]), Some(BUTTON_SET));
        assert_eq!(button_at([150.0, 60.0]), Some(BUTTON_DELETE));
        assert_eq!(button_at([0.0, 0.0]), None);
        let c = Choice::default();
        assert_eq!(c, Choice { channel: 1, public: true });
        assert_eq!(c.press(BUTTON_PREV).channel, CHANNELS, "wraps");
        assert_eq!(c.press(BUTTON_NEXT).channel, 2);
        assert_eq!(Choice { channel: CHANNELS, public: true }.press(BUTTON_NEXT).channel, 1);
        assert!(!c.press(BUTTON_PUBLIC).public);
        assert_eq!(c.press(BUTTON_SET), c, "set is the host's");
        let id = Choice { channel: 5, public: false }.identity(Kind::Inventory, BOB);
        assert_eq!(id, Identity::new(Kind::Inventory, "5", false, BOB));
        assert_eq!(channel_of("5"), Some(5));
        assert_eq!(channel_of("17"), None);
        assert_eq!(channel_of("0"), None);
        assert_eq!(channel_of("x"), None);
    }

    #[test]
    fn entangloporter_capacities_are_the_ultimate_tiers() {
        assert_eq!(ENERGY_CAP, 256_000_000);
        assert_eq!(FLUID_CAP, 256 * 1_000);
        assert_eq!(CHEMICAL_CAP, 8_192 * 1_000);
        assert!(ENERGY_WINDOW <= i64::from(i32::MAX) && FLUID_WINDOW < FLUID_CAP && CHEMICAL_WINDOW < CHEMICAL_CAP);
    }
}
