//! The host side of frequencies, the Teleporter and the Portable Teleporter (M08).
//! Rules and values are Horizon-free in `frequency.rs` and `teleporter.rs`; read their
//! headers for what is ported and from which Mekanism class.
//!
//! State:
//! - The frequency registry is one process-wide `State`, loaded from and saved to
//!   `<world>/mekanism_horizon/frequencies.txt` (`host.world_dir()`), keyed by the world's
//!   folder so a world switch loads the other world's file. It is written only when the world
//!   saves (`world_saved`, which the final save on close also fires), like Mekanism's
//!   `SavedData`: a Quantum Entangloporter's pool and the windows its members hold in their
//!   chunks are then the same tick's snapshot, so a restart neither doubles nor loses what was
//!   lent out. A crash loses the changes since the last save, as for the chunks.
//!   Without a world folder (no server) it stays in memory.
//! - Which teleporters are active is not saved: each ticking teleporter registers itself
//!   (`Registry::touch`), so only teleporters in loaded chunks are destinations, as in Mekanism.
//! - A block entity keeps its bound frequency (`mekanism_freq_*`) and the GUI's selection
//!   (`mekanism_sel_*`) in its NBT.
//!
//! Who gets teleported: a block entity's tick has no player, so `player_tick` (API 31, once a tick
//! for every living player, with the player acting) finds the player standing in a ready
//! teleporter's bounds. Only players are teleported: Horizon has no entity search in a box
//! and no mob teleport by position (`native_teleport_mob` takes an entity id).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Flow, Host, InventoryTickEvent, ItemBarEvent, ItemStack, ModContainer,
    ModEnergyBuffer, NativeScreen, ScreenDrawEvent, Tag, Text,
};

use crate::frequency::{self as freq, Choice, Error, Identity, Kind as FKind, Member, Pos, Registry};
use crate::teleporter::{self, Frame, Gate, Status};
use crate::{ENERGY, ITEM_ENERGY, cube, key, read_energy, storage, write_int};

pub(super) const FRAME: &str = "mekanism:teleporter_frame";
pub(super) const PORTABLE: &str = "mekanism:portable_teleporter";
/// Registered by `data/mekanism/horizon/menu/teleporter.json`.
const MENU: &str = "mekanism:teleporter";

const NAME: &str = "mekanism_freq_name";
const PUBLIC: &str = "mekanism_freq_public";
const OWNER: &str = "mekanism_freq_owner";
const SEL_CHANNEL: &str = "mekanism_sel_channel";
const SEL_PUBLIC: &str = "mekanism_sel_public";
const ITEM_CHANNEL: &str = "mekanism_channel";
const ITEM_PUBLIC: &str = "mekanism_public";
const CUSTOM_DATA: &str = "minecraft:custom_data";

const FOLDER: &str = "mekanism_horizon";
const FILE: &str = "frequencies.txt";
/// A teleporter not seen this long (milliseconds) is not a destination: it is in an unloaded chunk.
pub(super) const ACTIVE_MS: u64 = 3_000;

/// Menu data of the frequency panel, shared with the Quantum Entangloporter.
pub(super) const DATA_PANEL: usize = 4;
const DATA_STATUS: usize = 4;
const DATA_ENERGY: usize = 5;
pub(super) const DATA_LEN: usize = 6;

/// What a teleporter tells the players who step into it, kept from its tick.
#[derive(Clone)]
pub(super) struct Dest {
    id: Identity,
    frame: Option<Frame>,
    bounds: Option<([f64; 3], [f64; 3])>,
    target: Pos,
}

pub(super) struct State {
    dir: Option<PathBuf>,
    pub(super) registry: Registry,
    dests: BTreeMap<Member, Dest>,
    gates: BTreeMap<Member, Gate>,
}

impl State {
    fn open(dir: Option<PathBuf>) -> State {
        let text = dir.as_ref().and_then(|d| std::fs::read_to_string(d.join(FOLDER).join(FILE)).ok());
        State {
            dir,
            registry: text.map_or_else(Registry::new, |t| Registry::load(&t)),
            dests: BTreeMap::new(),
            gates: BTreeMap::new(),
        }
    }

    /// Write the registry if it changed.
    fn flush(&mut self) {
        if !self.registry.dirty {
            return;
        }
        let Some(dir) = &self.dir else { return };
        let folder = dir.join(FOLDER);
        let tmp = folder.join("frequencies.tmp");
        let written = std::fs::create_dir_all(&folder)
            .and_then(|()| std::fs::write(&tmp, self.registry.save()))
            .and_then(|()| std::fs::rename(&tmp, folder.join(FILE)));
        if written.is_ok() {
            self.registry.dirty = false;
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub(super) fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Runs `f` on the state of the world the host serves, then saves it if due. `f` must not call
/// the host (the lock is held), so it works on what the caller fetched before.
pub(super) fn with<R>(host: &mut dyn Host, f: impl FnOnce(&mut State, u64) -> R) -> R {
    let dir = host.world_dir();
    let now = now_ms();
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_none_or(|s| s.dir != dir) {
        if let Some(old) = guard.as_mut() {
            old.flush();
        }
        *guard = Some(State::open(dir));
    }
    let state = guard.as_mut().expect("opened above");
    f(state, now)
}

/// The world saved its chunks (`world_saved`): write the registry of that world now, so both
/// are the same tick's snapshot.
pub(super) fn save(host: &dyn Host) {
    let dir = host.world_dir();
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = guard.as_mut().filter(|s| s.dir == dir) {
        state.flush();
    }
}

// --- NBT of the bound frequency and the GUI's selection ---

pub(super) fn read_str(nbt: &Compound, name: &'static str) -> Option<String> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()).filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn write_str(nbt: &mut Compound, name: &'static str, value: &str) {
    let k = key(nbt, name);
    if read_str(nbt, name).as_deref() != Some(value) {
        nbt.insert(k, Tag::String(value.into()));
    }
}

fn read_byte(nbt: &Compound, name: &'static str) -> Option<bool> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Byte(b)) => Some(*b != 0),
        _ => None,
    }
}

fn write_byte(nbt: &mut Compound, name: &'static str, value: bool) {
    let k = key(nbt, name);
    let want = i8::from(value);
    if nbt.get(&k) != Some(&Tag::Byte(want)) {
        nbt.insert(k, Tag::Byte(want));
    }
}

/// The frequency the block is bound to.
pub(super) fn bound(nbt: &Compound, kind: FKind) -> Option<Identity> {
    let name = read_str(nbt, NAME)?;
    let public = read_byte(nbt, PUBLIC).unwrap_or(true);
    let owner = read_str(nbt, OWNER).and_then(|o| storage::uuid_from_hex(&o)).unwrap_or(0);
    Some(Identity::new(kind, &name, public, owner))
}

/// The channel and mode the panel shows.
pub(super) fn selection(nbt: &Compound) -> Choice {
    let channel = read_energy(nbt, SEL_CHANNEL);
    Choice {
        channel: u8::try_from(channel).ok().filter(|c| (1..=freq::CHANNELS).contains(c)).unwrap_or(1),
        public: read_byte(nbt, SEL_PUBLIC).unwrap_or(true),
    }
}

fn write_selection(nbt: &mut Compound, c: Choice) {
    write_int(nbt, SEL_CHANNEL, i32::from(c.channel));
    write_byte(nbt, SEL_PUBLIC, c.public);
}

/// `[selected channel, selected public, bound channel (0 = none), bound public]`.
pub(super) fn panel_data(nbt: &Compound, kind: FKind) -> [i32; 4] {
    let sel = selection(nbt);
    let (channel, public) = bound(nbt, kind)
        .map_or((0, 0), |b| (freq::channel_of(&b.name).map_or(0, i32::from), i32::from(b.public)));
    [i32::from(sel.channel), i32::from(sel.public), channel, public]
}

/// A panel button: previous/next channel and the mode only change the selection; Set creates the
/// selected frequency if it does not exist and binds the block to it; Delete removes it.
pub(super) fn panel_button(host: &mut dyn Host, nbt: &mut Compound, kind: FKind, id: i32) {
    let sel = selection(nbt);
    if !matches!(id, freq::BUTTON_SET | freq::BUTTON_DELETE) {
        write_selection(nbt, sel.press(id));
        return;
    }
    let Some(player) = host.acting_player() else { return };
    let identity = sel.identity(kind, player.uuid);
    let mode = if sel.public { "public" } else { "private" };
    let message = if id == freq::BUTTON_SET {
        let outcome = with(host, |s, _| {
            s.registry.create(&identity, &player.name)
        });
        write_str(nbt, NAME, &identity.name);
        write_byte(nbt, PUBLIC, identity.public);
        write_str(nbt, OWNER, &storage::uuid_to_hex(player.uuid));
        match outcome {
            Ok(()) => format!("Frequency {} ({mode}) created and set", identity.name),
            Err(_) => format!("Frequency {} ({mode}) set", identity.name),
        }
    } else {
        let outcome = with(host, |s, _| {
            s.registry.delete(&identity, player.uuid, player.permission)
        });
        match outcome {
            Ok(()) => format!("Frequency {} ({mode}) deleted", identity.name),
            Err(Error::NotOwner) => "Only its owner or an operator can delete a frequency".to_owned(),
            Err(_) => format!("There is no frequency {} ({mode})", identity.name),
        }
    };
    host.action_bar(player.uuid, Text::literal(message));
}

/// The panel's buttons and labels, drawn on a screen of 176 px width; `y` is the title row.
pub(super) fn draw_panel(host: &dyn Host, event: &mut ScreenDrawEvent, kind_name: &str) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let (channel, public, bound_channel, bound_public) = (d(0), d(1) != 0, d(2), d(3) != 0);
    let mode = |public: bool| if public { "Public" } else { "Private" };
    let mut button = |rect: [f32; 4], label: &str, color: u32| {
        let [bx, by, w, h] = rect;
        host.fill(event.batch, [x + bx, y + by, w, h], 0xFF262A2F);
        host.fill(event.batch, [x + bx + 1.0, y + by + 1.0, w - 2.0, h - 2.0], color);
        host.text(event.batch, label, [x + bx + 4.0, y + by + 3.0], 0xFFFFFF, true);
    };
    button(freq::PREV_RECT, "<", 0xFF6B7075);
    button(freq::NEXT_RECT, ">", 0xFF6B7075);
    button(freq::PUBLIC_RECT, mode(public), if public { 0xFF278447 } else { 0xFF6B7075 });
    button(freq::SET_RECT, "Set", 0xFF3C6EA8);
    button(freq::DELETE_RECT, "Delete", 0xFFAF4141);
    host.text(event.batch, &format!("{kind_name} channel {channel}"), [x + 32.0, y + 44.0], 0x333333, false);
    let line = if bound_channel == 0 {
        "Not bound: pick a channel, press Set".to_owned()
    } else {
        format!("Bound to {} channel {bound_channel}", mode(bound_public).to_lowercase())
    };
    host.text(event.batch, &line, [x + 8.0, y + 76.0], 0x404040, false);
}

/// The panel button under the mouse, for both blocks.
pub(super) fn panel_click(at: [f32; 2]) -> Option<i32> {
    freq::button_at(at)
}

// --- the Teleporter block ---

pub(super) fn container() -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(MENU.to_owned()),
    }
}

/// `StorageConfig.teleporter` in every side; the block takes energy and gives none.
pub(super) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: teleporter::ENERGY_CAPACITY,
        max_insert: teleporter::ENERGY_CAPACITY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

pub(super) fn screen() -> NativeScreen {
    NativeScreen {
        width: 176.0,
        height: 176.0,
        slots: vec![None],
        inventory_y: 94.0,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}

pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let title = host.screen_title().unwrap_or_else(|| "Teleporter".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let status = Status::from_code(event.data.get(DATA_STATUS).copied().unwrap_or(0));
    let color = if status.is_error() { 0xAF4141 } else { 0x278447 };
    host.text(event.batch, &format!("Status: {}", status.text()), [x + 8.0, y + 18.0], color, false);
    let energy = event.data.get(DATA_ENERGY).copied().unwrap_or(0);
    host.text(
        event.batch,
        &format!("Energy: {energy} / {} FE", teleporter::ENERGY_CAPACITY),
        [x + 8.0, y + 29.0],
        0x333333,
        false,
    );
    draw_panel(host, event, "Teleporter");
    host.text(event.batch, "Inventory", [x + 8.0, y + 84.0], 0x404040, false);
}

/// One teleporter tick (`TileEntityTeleporter.onUpdateServer`): find the frame, register as
/// active on the bound frequency and work out the status.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
    let pos = event.pos;
    let dimension = host.dimension().to_owned();
    let id = bound(&event.entity.nbt, FKind::Teleporter);
    let frame = {
        let world: &dyn Host = &*host;
        teleporter::find_frame(pos, &|p| &*world.block(p) == FRAME)
    };
    let energy = read_energy(&event.entity.nbt, ENERGY);
    let member = Member::new(&dimension, pos);
    let dest = Dest {
        id: id.clone().unwrap_or_else(|| Identity::new(FKind::Teleporter, "", true, 0)),
        frame,
        bounds: teleporter::bounds(pos, frame),
        target: teleporter::target_pos(pos, frame),
    };
    let status = with(host, |s, now| {
        let Some(id) = id.as_ref().filter(|i| s.registry.get(i).is_some()) else {
            s.registry.forget(&member);
            s.dests.remove(&member);
            return Status::NoFrequency;
        };
        s.registry.touch(id, &member, now);
        s.dests.insert(member.clone(), dest);
        let destination = s.registry.closest(id, &member, now, ACTIVE_MS).is_some();
        Status::of(true, frame.is_some(), destination, energy, teleporter::BASE_COST)
    });
    let [a, b, c, d] = panel_data(&event.entity.nbt, FKind::Teleporter);
    let data = vec![a, b, c, d, status.code(), energy.clamp(0, teleporter::ENERGY_CAPACITY)];
    if event.entity.data != data {
        event.entity.data = data;
    }
}

/// A teleporter was broken or replaced.
pub(super) fn forget(host: &mut dyn Host, pos: Pos) {
    let member = Member::new(host.dimension(), pos);
    with(host, |s, _| {
        s.registry.forget(&member);
        s.dests.remove(&member);
        s.gates.remove(&member);
    });
}

/// `TileEntityTeleporter.teleport` for the acting player: if they stand in the bounds of a ready
/// teleporter and have not just arrived there, pay from that teleporter and send them to the
/// closest active teleporter on its frequency.
pub(super) fn player_tick(host: &mut dyn Host) {
    let Some(p) = host.acting_player() else { return };
    let dimension = p.dimension.to_string();
    let plan = with(host, |s, now| {
        let here: Vec<Member> = s.dests.keys().filter(|m| m.dimension == dimension).cloned().collect();
        let mut plan = None;
        for member in here {
            let dest = s.dests[&member].clone();
            let Some(bounds) = dest.bounds else { continue };
            let gate = s.gates.entry(member.clone()).or_default();
            if !teleporter::inside(bounds, p.position) {
                gate.left(p.uuid);
                continue;
            }
            if plan.is_some() || !gate.may_send(p.uuid, now) {
                continue;
            }
            if !s.registry.active(&dest.id, now, ACTIVE_MS).contains(&member) {
                continue;
            }
            let Some(to) = s.registry.closest(&dest.id, &member, now, ACTIVE_MS) else { continue };
            let Some(target) = s.dests.get(&to).cloned() else { continue };
            plan = Some((member, to, target));
        }
        plan
    });
    let Some((from, to, target)) = plan else { return };
    let cost = teleporter::energy_cost(p.position, &dimension, to.pos, &to.dimension, 0);
    let have = host.mod_block_entity(from.pos).map_or(0, |e| read_energy(&e.nbt, ENERGY));
    if have < cost {
        host.action_bar(p.uuid, Text::literal(format!("Teleporter: needs {cost} FE, has {have} FE")));
        with(host, |s, _| s.gates.entry(from.clone()).or_default().hold(p.uuid));
        return;
    }
    host.change_mod_block_entity(from.pos, &mut |m| {
        let now = read_energy(&m.nbt, ENERGY);
        write_int(&mut m.nbt, ENERGY, (now - cost).max(0));
    });
    let at = [f64::from(target.target[0]) + 0.5, f64::from(target.target[1]), f64::from(target.target[2]) + 0.5];
    let yaw = target.frame.and_then(|f| f.dir.yaw()).unwrap_or(p.yaw);
    if host.teleport_player(p.uuid, &to.dimension, at, yaw, p.pitch) {
        with(host, |s, now| s.gates.entry(to.clone()).or_default().arrive(p.uuid, now));
    } else {
        host.change_mod_block_entity(from.pos, &mut |m| {
            let now = read_energy(&m.nbt, ENERGY);
            write_int(&mut m.nbt, ENERGY, (now + cost).min(teleporter::ENERGY_CAPACITY));
        });
        with(host, |s, _| s.gates.entry(from.clone()).or_default().hold(p.uuid));
    }
}

// --- the Portable Teleporter item ---

type Edit = Box<dyn FnOnce(&dyn Host, &mut ItemStack) + Send>;

/// Changes of a player's main-hand stack, applied in their next `inventory_tick`: the use hooks
/// give the stack by reference and `carried_stack` works only there (as `items.rs`).
static PENDING: Mutex<BTreeMap<u128, Vec<Edit>>> = Mutex::new(BTreeMap::new());

/// SNBT of the tags an item keeps in `custom_data`; any other kind is left out.
fn snbt(tag: &Tag) -> Option<String> {
    Some(match tag {
        Tag::Byte(n) => format!("{n}b"),
        Tag::Int(n) => n.to_string(),
        Tag::Long(n) => format!("{n}L"),
        Tag::String(s) => format!("{:?}", s.to_string_lossy()),
        Tag::Compound(c) => {
            let entries: Vec<_> = c
                .iter()
                .filter_map(|(k, v)| Some(format!("{:?}:{}", k.to_string_lossy(), snbt(v)?)))
                .collect();
            format!("{{{}}}", entries.join(","))
        }
        _ => return None,
    })
}

/// `stack` with its custom data after `change`: rebuilt from SNBT, as `items.rs` does.
fn edit_data(host: &dyn Host, stack: &mut ItemStack, change: impl FnOnce(&mut Compound)) {
    let mut data = stack.custom_data().cloned().unwrap_or_default();
    change(&mut data);
    let Some(data) = snbt(&Tag::Compound(data)) else { return };
    let text = format!("{{id:{:?},count:{},components:{{{CUSTOM_DATA:?}:{data}}}}}", &*stack.item, stack.count);
    if let Some(next) = host.item_from_snbt(&text) {
        *stack = next;
    }
}

fn edit_held(host: &mut dyn Host, change: impl FnOnce(&dyn Host, &mut ItemStack) + Send + 'static) {
    let Some(player) = host.acting_player() else { return };
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).entry(player.uuid).or_default().push(Box::new(change));
}

/// Apply the queued changes to the main-hand stack.
pub(super) fn inventory_tick(host: &mut dyn Host, event: &InventoryTickEvent) {
    if !event.selected {
        return;
    }
    let Some(player) = host.acting_player() else { return };
    let edits = PENDING.lock().unwrap_or_else(|e| e.into_inner()).remove(&player.uuid).unwrap_or_default();
    if edits.is_empty() {
        return;
    }
    let Some(mut stack) = host.carried_stack(event.slot).cloned() else { return };
    for edit in edits {
        edit(&*host, &mut stack);
    }
    if let Some(slot) = host.carried_stack(event.slot) {
        *slot = stack;
    }
}

/// The item's channel and mode.
fn item_choice(stack: &ItemStack) -> Choice {
    let Some(d) = stack.custom_data() else { return Choice::default() };
    let channel = read_energy(d, ITEM_CHANNEL);
    Choice {
        channel: u8::try_from(channel).ok().filter(|c| (1..=freq::CHANNELS).contains(c)).unwrap_or(1),
        public: !matches!(d.get(&key(d, ITEM_PUBLIC)), Some(Tag::Int(0))),
    }
}

fn item_energy(stack: &ItemStack) -> i32 {
    stack.custom_data().map_or(0, |d| read_energy(d, ITEM_ENERGY))
}

fn mode_name(public: bool) -> &'static str {
    if public { "public" } else { "private" }
}

/// Using the item in the air: sneaking steps through the channels (public 1 to 16, then
/// private 1 to 16), else it teleports to the closest teleporter on its frequency. Horizon's
/// `open_menu` is LAN-only, so there is no frequency screen for the item.
pub(super) fn use_portable(host: &mut dyn Host, stack: &ItemStack) -> Flow {
    let Some(p) = host.acting_player() else { return Flow::Continue };
    let choice = item_choice(stack);
    if host.sneaking() {
        let next = if choice.channel >= freq::CHANNELS {
            Choice { channel: 1, public: !choice.public }
        } else {
            Choice { channel: choice.channel + 1, ..choice }
        };
        edit_held(host, move |h, s| {
            edit_data(h, s, |d| {
                d.insert(key(d, ITEM_CHANNEL), Tag::Int(i32::from(next.channel)));
                d.insert(key(d, ITEM_PUBLIC), Tag::Int(i32::from(next.public)));
            });
        });
        host.action_bar(
            p.uuid,
            Text::literal(format!("Portable Teleporter: {} channel {}", mode_name(next.public), next.channel)),
        );
        return Flow::Cancel;
    }
    let dimension = p.dimension.to_string();
    let id = choice.identity(FKind::Teleporter, p.uuid);
    let me = Member::new(&dimension, [p.position[0].floor() as i32, p.position[1].floor() as i32, p.position[2].floor() as i32]);
    let (exists, found) = with(host, |s, now| {
        let to = s.registry.closest(&id, &me, now, ACTIVE_MS);
        let target = to.as_ref().and_then(|t| s.dests.get(t).cloned());
        (s.registry.get(&id).is_some(), to.zip(target))
    });
    let say = |host: &mut dyn Host, text: String| host.action_bar(p.uuid, Text::literal(text));
    if !exists {
        say(host, format!("No {} frequency {} exists: set it on a Teleporter", mode_name(choice.public), choice.channel));
        return Flow::Cancel;
    }
    let Some((to, target)) = found else {
        say(host, "No active teleporter on this frequency".to_owned());
        return Flow::Cancel;
    };
    let creative = p.game_mode == 1;
    let cost = if creative { 0 } else { teleporter::energy_cost(p.position, &dimension, to.pos, &to.dimension, 0) };
    let have = item_energy(stack);
    if have < cost {
        say(host, format!("Not enough energy: needs {cost} FE, has {have} FE"));
        return Flow::Cancel;
    }
    let at = [f64::from(target.target[0]) + 0.5, f64::from(target.target[1]), f64::from(target.target[2]) + 0.5];
    let yaw = target.frame.and_then(|f| f.dir.yaw()).unwrap_or(p.yaw);
    if host.teleport_player(p.uuid, &to.dimension, at, yaw, p.pitch) {
        with(host, |s, now| s.gates.entry(to.clone()).or_default().arrive(p.uuid, now));
        if cost > 0 {
            edit_held(host, move |h, s| {
                edit_data(h, s, |d| {
                    let now = read_energy(d, ITEM_ENERGY);
                    d.insert(key(d, ITEM_ENERGY), Tag::Int((now - cost).max(0)));
                });
            });
        }
    } else {
        say(host, "The teleport was refused".to_owned());
    }
    Flow::Cancel
}

pub(super) fn tooltip(stack: &ItemStack) -> Vec<(Text, u32)> {
    let grey = 0xFF_AAAAAA;
    let c = item_choice(stack);
    vec![
        (Text::literal(format!("Frequency: {} channel {}", mode_name(c.public), c.channel)), grey),
        (
            Text::literal(format!(
                "Stored: {} / {} FE",
                cube::si(i64::from(item_energy(stack))),
                cube::si(i64::from(teleporter::PORTABLE_CAPACITY))
            )),
            grey,
        ),
    ]
}

pub(super) fn bar(event: &mut ItemBarEvent<'_>) {
    if &*event.stack.item != PORTABLE {
        return;
    }
    let fill = (item_energy(event.stack) as f32 / teleporter::PORTABLE_CAPACITY as f32).clamp(0.0, 1.0);
    *event.bar = Some((((fill * 13.0).round() as i32).max(i32::from(fill > 0.0)), 0x2ED8FF));
}
