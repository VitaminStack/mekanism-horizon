//! The host side of the generators (Mekanism: Generators, M04): ticks, world
//! reads, containers, screens, multiblocks and drawing. Their Horizon-free
//! rules and values are in `generator.rs`.
//!
//! They read the world through `Host::block`, `biome`, `light`, `weather`,
//! `day_time` and `dimension` (API 20/97), and push their energy into the
//! neighbouring cube or machine on their output side (`give`).

use horizon_client::mod_api::{
    BlockEntityDraw, BlockEntityTickEvent, Compound, Host, ItemStack, ModContainer, ModEnergyBuffer,
    ModFluidTanks, NativeScreen, ScreenDrawEvent, Tag,
};

use super::{
    MekanismNative, SOUND_WEATHER, chemicals, cube, facing, gauge, generator, give, key, offset, read_energy,
    shrink, tanks, write_int, ENERGY,
};

/// Block entity NBT of the Heat Generator: stored heat (Tag::Double) and lava in mB.
/// The Bio Generator keeps its bioethanol in mB as `mekanism_bio_fuel`.
const BIO_FUEL: &str = "mekanism_bio_fuel";
pub(super) const HEAT: &str = "mekanism_heat";
const LAVA: &str = "mekanism_lava";
/// A Wind Generator turning its blades, sent to clients (1 or 0).
pub(super) const WIND_ACTIVE: &str = "mekanism_wind_active";

pub(super) const ALL: [&str; 6] =
    [generator::HEAT, generator::SOLAR, generator::BIO, generator::ADVANCED_SOLAR, generator::WIND, generator::GAS];

/// The generator `block` is, as its id constant.
pub(super) fn find(block: &str) -> Option<&'static str> {
    ALL.into_iter().find(|g| *g == block)
}

/// Registered by `data/mekanism/horizon/menu/bounding_block.json`: a
/// multiblock's screen, opened from any of its bounding blocks.
const BOUNDING_MENU: &str = generator::BOUNDING;
/// Block entity NBT of a bounding block: its main block's position.
const MAIN: &str = "mekanism_main";
/// A bounding block's menu data: its main block's (`SOLAR_DATA_*`), then
/// which generator that is (`ALL` index + 1; 0 without a main block).
const BOUNDING_DATA_KIND: usize = generator::SOLAR_DATA_LEN;

/// The name a generator's screen shows when it is opened from a bounding
/// block (whose own block name the screen title would be).
fn name(g: &str) -> &'static str {
    match g {
        generator::WIND => "Wind Generator",
        generator::ADVANCED_SOLAR => "Advanced Solar Generator",
        generator::HEAT => "Heat Generator",
        generator::BIO => "Bio-Generator",
        generator::GAS => "Gas-Burning Generator",
        _ => "Solar Generator",
    }
}

/// Mekanism's `BlockBounding` opens the main block's screen. Horizon gives a
/// block's menu per block type and has no call that opens another block's
/// menu (docs/upstream/horizon-block-menu-host.md), so a bounding block has
/// a menu of its own that mirrors its main block's data.
pub(super) fn bounding_container() -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(BOUNDING_MENU.to_owned()),
    }
}

/// The multiblock a bounding block belongs to: the one saved, while it is
/// still there, else the one found around it.
fn main_of(host: &dyn Host, nbt: &Compound, pos: [i32; 3]) -> Option<(&'static str, [i32; 3])> {
    let saved = match nbt.get(&key(nbt, MAIN)) {
        Some(Tag::IntArray(v)) if v.len() == 3 => Some([v[0], v[1], v[2]]),
        _ => None,
    };
    saved
        .and_then(|m| {
            let g = host.mod_block_entity(m).and_then(|e| find(&e.id))?;
            generator::is_multiblock(g).then_some((g, m))
        })
        .or_else(|| bounding_main(host, pos))
}

/// A bounding block's tick: copy its main block's menu data.
pub(super) fn bounding_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
    let main = main_of(host, &event.entity.nbt, event.pos);
    let mut data = Vec::new();
    if let Some((g, m)) = main {
        let k = key(&event.entity.nbt, MAIN);
        let at = Tag::IntArray(m.to_vec());
        if event.entity.nbt.get(&k) != Some(&at) {
            event.entity.nbt.insert(k, at);
        }
        if let Some(e) = host.mod_block_entity(m) {
            data = e.data.clone();
            data.resize(BOUNDING_DATA_KIND, 0);
            data.push(ALL.iter().position(|a| *a == g).map_or(0, |i| i as i32 + 1));
        }
    }
    if event.entity.data != data {
        event.entity.data = data;
    }
}

/// The generator a bounding block's menu data shows.
pub(super) fn bounding_generator(data: &[i32]) -> Option<&'static str> {
    let kind = data.get(BOUNDING_DATA_KIND)?.checked_sub(1)?;
    ALL.get(usize::try_from(kind).ok()?).copied()
}

/// Heat and Bio Generator: hoppers fill the fuel slot from any side. The
/// others: one hidden placeholder slot, as the cube's.
pub(super) fn container(g: &str) -> ModContainer {
    let (slots, menu) = match g {
        generator::HEAT => (vec![0], generator::HEAT_MENU),
        generator::BIO => (vec![0], generator::BIO_MENU),
        generator::ADVANCED_SOLAR => (Vec::new(), generator::ADVANCED_SOLAR_MENU),
        generator::WIND => (Vec::new(), generator::WIND_MENU),
        generator::GAS => (Vec::new(), generator::GAS_MENU),
        _ => (Vec::new(), generator::SOLAR_MENU),
    };
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| slots.clone()),
        inputs: slots.clone(),
        outputs: Vec::new(),
        menu: Some(menu.to_owned()),
    }
}

/// Generators only give. Horizon's faces are per block type, not per facing:
/// other mods may take a generator's energy on any side but the Solar
/// Generator's (Mekanism: the front, and the bottom of the solar panels).
pub(super) fn energy_buffer(g: &str) -> ModEnergyBuffer {
    let (capacity, faces) = match g {
        generator::HEAT => (generator::HEAT_STORAGE, [true; 6]),
        generator::BIO => (generator::BIO_STORAGE, [true; 6]),
        generator::SOLAR => {
            let mut bottom = [false; 6];
            bottom[cube::DOWN] = true;
            (generator::SOLAR_STORAGE, bottom)
        }
        _ => (generator::panel_storage(g), [true; 6]),
    };
    ModEnergyBuffer { capacity, max_insert: 0, max_extract: capacity, insert_faces: [false; 6], extract_faces: faces }
}

/// The Gas-Burning Generator's one chemical tank, reached from every side
/// (Mekanism: every side but the front; Horizon's faces are per block type).
/// Input only: nothing takes the fuel back out.
pub(super) fn gas_fluids() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![generator::GAS_TANK],
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: Vec::new(),
    }
}

pub(super) fn screen(g: &str) -> NativeScreen {
    let fuel = match g {
        generator::HEAT => Some(generator::HEAT_FUEL_SLOT),
        generator::BIO => Some(generator::BIO_FUEL_SLOT),
        _ => None,
    };
    NativeScreen {
        width: generator::WIDTH,
        height: generator::HEIGHT,
        slots: vec![fuel],
        inventory_y: generator::INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}

pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, g: &str) {
    match g {
        generator::HEAT => draw_heat(host, event),
        generator::BIO => draw_bio(host, event),
        generator::GAS => draw_gas(host, event),
        _ => draw_solar(host, event, g),
    }
}

pub(super) fn tooltip(g: &str, data: &[i32], at: [f32; 2]) -> Vec<String> {
    match g {
        generator::HEAT => generator::heat_tooltip(data, at),
        generator::BIO => generator::bio_tooltip(data, at),
        generator::GAS => {
            let id = data
                .get(generator::GAS_DATA_FUEL)
                .and_then(|i| usize::try_from(*i).ok()?.checked_sub(1))
                .and_then(chemicals::id_at);
            let density = id.as_deref().and_then(generator::gas_fuel).map_or(0, |f| f.0);
            generator::gas_tooltip(data, at, &id.as_deref().map_or_else(String::new, chemicals::name), density)
        }
        _ => generator::solar_tooltip(g, data, at),
    }
}

impl MekanismNative {
    pub(super) fn generator_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent, g: &str) {
        match g {
            generator::HEAT => self.heat_tick(host, event),
            generator::SOLAR => self.solar_tick(host, event),
            generator::BIO => self.bio_tick(host, event),
            generator::GAS => self.gas_tick(host, event),
            generator::ADVANCED_SOLAR => self.advanced_solar_tick(host, event),
            _ => self.wind_tick(host, event),
        }
    }

    /// One Heat Generator tick: the fuel slot fills the lava tank (a lava
    /// bucket whole, leaving the bucket; a furnace fuel by its burn time), then
    /// `generator::Heat::tick`, then the energy goes out of the front.
    fn heat_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let entity = &*event.entity;
        let mut items: Vec<Option<ItemStack>> = entity.items.clone();
        items.resize(1, None);
        let ambient = generator::ambient_temp(generator::climate(host.biome(pos).as_deref()).0 as f64);
        let mut state = generator::Heat {
            heat: read_double(&entity.nbt, HEAT).unwrap_or_else(|| generator::Heat::new(ambient).heat),
            energy: read_energy(&entity.nbt, ENERGY).clamp(0, generator::HEAT_STORAGE),
            lava: read_energy(&entity.nbt, LAVA).clamp(0, generator::LAVA_TANK),
        };

        if let Some(stack) = &items[0] {
            if &*stack.item == "minecraft:lava_bucket" {
                if state.lava + generator::LAVA_TANK <= generator::LAVA_TANK {
                    if let Some(bucket) = host.item_from_snbt("{id:\"minecraft:bucket\",count:1}") {
                        state.lava += generator::LAVA_TANK;
                        items[0] = Some(bucket);
                    }
                }
            } else if let Some(lava) = burn_time(host, stack).map(generator::fuel_lava) {
                if lava > 0 && state.lava + lava <= generator::LAVA_TANK {
                    state.lava += lava;
                    items[0] = shrink(stack, 1);
                }
            }
        }

        let lava_sides = (0..6).filter(|&d| host.block(offset(pos, d)) == "minecraft:lava").count() as u32;
        let nether = host.dimension() == "minecraft:the_nether";
        let tick = state.tick(generator::boost(lava_sides, nether), ambient);
        let f = facing(host, pos);
        state.energy -= give(host, pos, f, state.energy, state.energy);
        self.hum(host, pos, generator::HEAT_SOUND, generator::HEAT_SOUND_TICKS, tick.active);
        // The model glows while burning; set the property only on change.
        let want = if tick.active { "true" } else { "false" };
        if host.block_property(pos, "active").as_deref() != Some(want) {
            host.set_block_property(pos, "active", want);
        }

        let e = &mut *event.entity;
        if e.items != items {
            e.items = items;
        }
        write_int(&mut e.nbt, ENERGY, state.energy);
        write_int(&mut e.nbt, LAVA, state.lava);
        let k = key(&e.nbt, HEAT);
        let heat = Tag::Double(state.heat.to_bits());
        if e.nbt.get(&k) != Some(&heat) {
            e.nbt.insert(k, heat);
        }
        let data = generator::heat_menu_data(&state, &tick, f);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Bio Generator tick, as Mekanism's `TileEntityBioGenerator`: fill
    /// the tank from the fuel slot, burn 1 mB for 25 FE, push out of the front.
    fn bio_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let entity = &*event.entity;
        let mut items: Vec<Option<ItemStack>> = entity.items.clone();
        items.resize(1, None);
        let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, generator::BIO_STORAGE);
        let mut fuel = read_energy(&entity.nbt, BIO_FUEL).clamp(0, generator::BIO_TANK);

        if let Some(stack) = &items[0] {
            let per_item = if host.server_item_in_tag(stack, generator::BIO_FUEL_TAG).unwrap_or(false) {
                generator::BIO_FUEL_PER_ITEM
            } else if host.server_item_in_tag(stack, generator::BIO_FUEL_BLOCK_TAG).unwrap_or(false) {
                9 * generator::BIO_FUEL_PER_ITEM
            } else {
                0
            };
            if generator::bio_fits(fuel, per_item) {
                fuel += per_item;
                items[0] = shrink(stack, 1);
            }
        }

        let active = generator::bio_tick(&mut fuel, &mut energy);
        let f = facing(host, pos);
        energy -= give(host, pos, f, energy, energy);
        self.hum(host, pos, generator::BIO_SOUND, generator::BIO_SOUND_TICKS, active);

        let e = &mut *event.entity;
        if e.items != items {
            e.items = items;
        }
        write_int(&mut e.nbt, ENERGY, energy);
        write_int(&mut e.nbt, BIO_FUEL, fuel);
        let data = generator::bio_menu_data(energy, fuel, active, f);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Gas-Burning Generator tick, as Mekanism's `TileEntityGasGenerator`:
    /// burn the tank's fuel (`generator::gas_tick`), push the energy out of the front.
    fn gas_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let (fluid, tank) = tanks::read_fluid(&event.entity.nbt);
        let mut amount = tank.clamp(0, generator::GAS_TANK);
        let mut energy = read_energy(&event.entity.nbt, ENERGY).clamp(0, generator::GAS_STORAGE);
        let fuel = fluid.as_deref().and_then(generator::gas_fuel);
        let used = fuel.map_or(0, |f| generator::gas_tick(&mut amount, &mut energy, f));
        let f = facing(host, pos);
        energy -= give(host, pos, f, energy, energy);
        self.hum(host, pos, generator::GAS_SOUND, generator::GAS_SOUND_TICKS, used > 0);

        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, energy);
        tanks::write_fluid(&mut e.nbt, fluid.as_deref(), amount);
        let kind = fluid.as_deref().filter(|_| amount > 0).and_then(chemicals::index).map_or(0, |i| i as i32 + 1);
        let data = generator::gas_menu_data(energy, amount, kind, used, f);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Advanced Solar Generator tick: its nine sky checks are the 3 x 3
    /// above the panel (the sky light over each bounding block of its top).
    fn advanced_solar_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let storage = generator::ADVANCED_SOLAR_STORAGE;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, storage);
        let dimension = host.dimension();
        let sky_light = dimension != "minecraft:the_nether" && dimension != "minecraft:the_end";
        let (rain, thunder) = host.weather();
        let darkness = generator::sky_darkness(host.day_time(), rain, thunder);
        let seeing = if generator::sees_sun(sky_light, darkness, 15) {
            (-1..=1)
                .flat_map(|x| (-1..=1).map(move |z| [pos[0] + x, pos[1] + 3, pos[2] + z]))
                .filter(|&p| host.light(p).1 >= 15)
                .count()
        } else {
            0
        };
        let peak = generator::peak_multiplier(generator::climate(host.biome(pos).as_deref()));
        let produced = generator::advanced_solar_production(seeing, darkness, peak).min(storage - stored);
        stored += produced;
        let f = facing(host, pos);
        stored -= give(host, pos, f, stored, stored);
        stored -= give(host, pos, cube::DOWN, stored, stored);
        self.hum(host, pos, generator::SOLAR_SOUND, generator::SOLAR_SOUND_TICKS, produced > 0);

        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, stored);
        let data = generator::solar_menu_data(stored, produced, seeing > 4, f, peak);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Wind Generator tick: the output rises with the height of its top
    /// (four blocks up), which must see the sky.
    fn wind_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let storage = generator::WIND_STORAGE;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, storage);
        let dimension = host.dimension().to_owned();
        let top = [pos[0], pos[1] + 4, pos[2]];
        let sees_sky = host.light([top[0], top[1] + 1, top[2]]).1 >= 15;
        let multiplier = generator::wind_multiplier(top[1], sees_sky, &dimension);
        let produced = generator::wind_generation(multiplier).min(storage - stored);
        stored += produced;
        let f = facing(host, pos);
        stored -= give(host, pos, f, stored, stored);
        stored -= give(host, pos, cube::DOWN, stored, stored);
        let active = multiplier > 0.0;
        self.hum_as(host, top, generator::WIND_SOUND, generator::WIND_SOUND_TICKS, active, SOUND_WEATHER);

        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, stored);
        if read_energy(&e.nbt, WIND_ACTIVE) != active as i32 {
            write_int(&mut e.nbt, WIND_ACTIVE, active as i32);
            host.block_entity_sync(pos);
        }
        let data = generator::solar_menu_data(stored, produced, active, f, multiplier as f32);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Solar Generator tick: produce while it sees the sun, then the energy
    /// goes out of the bottom.
    fn solar_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, generator::SOLAR_STORAGE);
        let dimension = host.dimension();
        let sky_light = dimension != "minecraft:the_nether" && dimension != "minecraft:the_end";
        let (rain, thunder) = host.weather();
        let darkness = generator::sky_darkness(host.day_time(), rain, thunder);
        // The panel is a full block here: the sky light it gets is the block's above it.
        let sees = generator::sees_sun(sky_light, darkness, host.light(offset(pos, cube::UP)).1);
        let peak = generator::peak_multiplier(generator::climate(host.biome(pos).as_deref()));
        let produced = generator::solar_production(sees, darkness, peak).min(generator::SOLAR_STORAGE - stored);
        stored += produced;
        stored -= give(host, pos, cube::DOWN, stored, stored);
        self.hum(host, pos, generator::SOLAR_SOUND, generator::SOLAR_SOUND_TICKS, produced > 0);

        let f = facing(host, pos);
        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, stored);
        let data = generator::solar_menu_data(stored, produced, sees, f, peak);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }
}

/// A furnace fuel's burn time: vanilla's fuels by id, then by item tag.
pub(super) fn burn_time(host: &dyn Host, stack: &ItemStack) -> Option<i32> {
    generator::fuel_burn_time(&stack.item).or_else(|| {
        generator::FUEL_TAGS
            .iter()
            .find(|(tag, _)| host.server_item_in_tag(stack, tag).unwrap_or(false))
            .map(|&(_, t)| t)
    })
}

pub(super) fn read_double(nbt: &Compound, name: &'static str) -> Option<f64> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits)).filter(|v| v.is_finite() && *v >= 0.0),
        _ => None,
    }
}

pub(super) fn add(pos: [i32; 3], o: [i32; 3]) -> [i32; 3] {
    [pos[0] + o[0], pos[1] + o[1], pos[2] + o[2]]
}

pub(super) fn is_air(host: &dyn Host, pos: [i32; 3]) -> bool {
    let b = host.block(pos);
    b == "minecraft:air" || b == "minecraft:cave_air" || b == "minecraft:void_air"
}

/// `Block.UPDATE_ALL`: neighbours are updated and clients told, block by block.
const UPDATE_ALL: i32 = 3;

/// Sets `block`'s default state at `parts` (only where `only` is, if given),
/// one block at a time through the level, as Mekanism's `setBlock` does.
/// Not `Host::edit_blocks`: its bulk path sends the chunk whole later, and
/// inside a player's break that left clients a stale copy of the multiblock.
pub(super) fn set_parts(host: &mut dyn Host, parts: &[[i32; 3]], block: &str, only: Option<&str>) {
    let Some(state) = host.default_state(block) else { return };
    let dimension = host.dimension().to_owned();
    for &p in parts {
        if only.is_none_or(|o| host.block(p) == o) {
            host.set_block(&dimension, p, state, UPDATE_ALL);
        }
    }
}

/// Mekanism's `HandleBoundingBlock`: the rest of a multiblock's shape is
/// bounding blocks. A placement without room for them is undone and the main
/// block drops as an item, as close as Horizon gets to vanilla's refused placement.
pub(super) fn place_multiblock(host: &mut dyn Host, pos: [i32; 3], block: &str) {
    let parts: Vec<[i32; 3]> = generator::bounding_offsets(block).into_iter().map(|o| add(pos, o)).collect();
    if !parts.iter().all(|&p| is_air(host, p)) {
        host.destroy_block(pos, None, false);
        return;
    }
    set_parts(host, &parts, generator::BOUNDING, None);
}

/// Removes a multiblock's bounding blocks (only bounding blocks, leaving `except`).
pub(super) fn remove_bounding(host: &mut dyn Host, main: [i32; 3], block: &str, except: Option<[i32; 3]>) {
    let parts: Vec<[i32; 3]> = generator::bounding_offsets(block)
        .into_iter()
        .map(|o| add(main, o))
        .filter(|&p| Some(p) != except)
        .collect();
    set_parts(host, &parts, "minecraft:air", Some(generator::BOUNDING));
}

/// The main block a bounding block at `pos` belongs to.
pub(super) fn bounding_main(host: &dyn Host, pos: [i32; 3]) -> Option<(&'static str, [i32; 3])> {
    generator::main_candidates(pos)
        .into_iter()
        .find(|&(block, main)| host.mod_block_entity(main).is_some_and(|e| &*e.id == block))
}

/// A vertical energy bar with its own fill colour.
pub(super) fn energy_bar(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, origin: [f32; 2], energy: i32, capacity: i32) {
    let [x, y, w, h] = generator::ENERGY_BAR;
    gauge(host, batch, [origin[0] + x, origin[1] + y, w, h], energy as f32 / capacity as f32, 0xFF50C878);
}

/// Turn of a facing about y, counterclockwise from above (the blockstate's `y` is clockwise).
pub(super) fn facing_degrees(dir: usize) -> f32 {
    match dir {
        cube::SOUTH => 180.0,
        cube::WEST => 90.0,
        cube::EAST => -90.0,
        _ => 0.0,
    }
}

/// `v` turned `degrees` counterclockwise about y (seen from above) through the block's centre.
pub(super) fn turn_y(v: [f32; 3], degrees: f32, about: f32) -> [f32; 3] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (x, z) = (v[0] - about, v[2] - about);
    [about + x * cos + z * sin, v[1], about - x * sin + z * cos]
}

/// The Wind Generator's tower and blades, turned to its facing; the blades
/// spin about the hub while it runs. Kept until the block entity changes.
pub(super) fn draw_wind(host: &dyn Host, pos: [i32; 3], active: bool, draw: &mut BlockEntityDraw) {
    let turn = facing_degrees(facing(host, pos));
    let speed = if active { generator::wind_rotor_speed(pos[1], host.dimension()) } else { 0.0 };
    // `spin` takes the axis and pivot in the block's own coordinates, whatever the pose.
    let axis = turn_y([0.0, 0.0, 1.0], turn, 0.0);
    let hub = turn_y(generator::WIND_HUB, turn, 0.5);
    draw.push();
    draw.rotate_about([0.0, 1.0, 0.0], turn, [0.5, 0.0, 0.5]);
    for blades in [false, true] {
        if blades {
            draw.spin(axis, hub, speed, 0.0);
        }
        for (model, moved, _) in generator::WIND_PARTS.iter().filter(|p| p.2 == blades) {
            draw.push();
            draw.translate([-moved[0], -moved[1], -moved[2]]);
            host.draw_model(draw, model, 0xFFFFFF);
            draw.pop();
        }
    }
    draw.still();
    draw.pop();
    draw.keep();
}

/// The Advanced Solar Generator's upper pole and panels, one block above its
/// main block and turned to its facing (its block model holds only the foot).
pub(super) fn draw_advanced_solar_top(host: &dyn Host, pos: [i32; 3], draw: &mut BlockEntityDraw) {
    draw.push();
    draw.rotate_about([0.0, 1.0, 0.0], facing_degrees(facing(host, pos)), [0.5, 0.0, 0.5]);
    draw.translate([0.0, 1.0, 0.0]);
    host.draw_model(draw, generator::ADVANCED_SOLAR_TOP, 0xFFFFFF);
    draw.pop();
    draw.keep();
}

/// The Bio Generator's screen, in Mekanism's layout: fuel bar at the left
/// edge, fuel slot, energy bar right.
pub(super) fn draw_bio(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| "Bio-Generator".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let [bx, by, bw, bh] = generator::BIO_FUEL_BAR;
    let fuel = d(generator::BIO_DATA_FUEL) as f32 / generator::BIO_TANK as f32;
    gauge(host, event.batch, [x + bx, y + by, bw, bh], fuel, 0xFFC8B43C);
    energy_bar(host, event.batch, event.origin, d(generator::BIO_DATA_ENERGY), generator::BIO_STORAGE);
    let [sx, sy, sw, sh] = generator::BIO_SCREEN;
    host.fill(event.batch, [x + sx, y + sy, sw, sh], 0xFF242A2F);
    host.text(event.batch, &format!("{} FE/t", d(generator::BIO_DATA_PRODUCING)), [x + sx + 4.0, y + sy + 4.0], 0x3FA9F5, false);
    host.text(event.batch, &format!("{} mB", d(generator::BIO_DATA_FUEL)), [x + sx + 4.0, y + sy + 16.0], 0x3FA9F5, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The Gas-Burning Generator's screen: the fuel gauge (in the chemical's
/// colour) in the middle, Mekanism's burn rate below, the energy bar right.
pub(super) fn draw_gas(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = match host.screen_title() {
        Some(t) if event.block != generator::BOUNDING => t,
        _ => name(generator::GAS).to_owned(),
    };
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let color = usize::try_from(d(generator::GAS_DATA_FUEL))
        .ok()
        .and_then(|i| i.checked_sub(1))
        .and_then(chemicals::entry)
        .map_or(0, |e| e.1);
    let [gx, gy, gw, gh] = generator::GAS_GAUGE;
    let fill = d(generator::GAS_DATA_AMOUNT) as f32 / generator::GAS_TANK as f32;
    gauge(host, event.batch, [x + gx, y + gy, gw, gh], fill, color);
    energy_bar(host, event.batch, event.origin, d(generator::GAS_DATA_ENERGY), generator::GAS_STORAGE);
    host.text(
        event.batch,
        &format!("Burn rate: {} mB/t", d(generator::GAS_DATA_USED)),
        [x + 8.0, y + generator::INVENTORY_Y - 22.0],
        0x404040,
        false,
    );
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The Heat Generator's screen: fuel slot left, lava gauge in the middle,
/// energy bar right; the temperature under the fuel slot.
pub(super) fn draw_heat(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| "Heat Generator".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let [gx, gy, gw, gh] = generator::HEAT_LAVA_GAUGE;
    let lava = d(generator::HEAT_DATA_LAVA) as f32 / generator::LAVA_TANK as f32;
    gauge(host, event.batch, [x + gx, y + gy, gw, gh], lava, 0xFFE8641E);
    energy_bar(host, event.batch, event.origin, d(generator::HEAT_DATA_ENERGY), generator::HEAT_STORAGE);
    host.text(event.batch, &format!("{} K", d(generator::HEAT_DATA_TEMP)), [x + 8.0, y + 58.0], 0x404040, false);
    host.text(event.batch, &format!("{} FE/t", d(generator::HEAT_DATA_PRODUCING)), [x + 124.0, y + 6.0], 0x404040, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The Solar Generator's screen: Mekanism's inner screen with energy and
/// production, the sun indicator left of it, the energy bar right.
pub(super) fn draw_solar(host: &dyn Host, event: &mut ScreenDrawEvent, g: &str) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = match host.screen_title() {
        Some(t) if event.block != generator::BOUNDING => t,
        _ => name(g).to_owned(),
    };
    let storage = generator::panel_storage(g);
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let [sx, sy, sw, sh] = generator::SOLAR_SCREEN;
    host.fill(event.batch, [x + sx, y + sy, sw, sh], 0xFF1C2620);
    let energy = d(generator::SOLAR_DATA_ENERGY);
    host.text(
        event.batch,
        &format!("{} / {} FE", cube::si(energy as i64), cube::si(storage as i64)),
        [x + sx + 4.0, y + sy + 6.0],
        0x3CFE9A,
        false,
    );
    host.text(
        event.batch,
        &format!("{} FE/t", d(generator::SOLAR_DATA_PRODUCING)),
        [x + sx + 4.0, y + sy + 22.0],
        0x3CFE9A,
        false,
    );
    let [ix, iy, iw, ih] = generator::SOLAR_SUN;
    let sun = if d(generator::SOLAR_DATA_SEES_SUN) != 0 { 0xFFFFD34D } else { 0xFF3A3F44 };
    host.fill(event.batch, [x + ix, y + iy, iw, ih], 0xFF262A2F);
    host.fill(event.batch, [x + ix + 2.0, y + iy + 2.0, iw - 4.0, ih - 4.0], sun);
    energy_bar(host, event.batch, event.origin, energy, storage);
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}
