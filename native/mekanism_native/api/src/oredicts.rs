//! The host side of the Oredictionificator (M08): tick, container, screen and buttons.
//! Rules, layout and menu data are Horizon-free in `oredict.rs`.
//!
//! Filters are saved in the block entity NBT as one string, `mekanism_od_filters`:
//! `tag,output,enabled` entries joined by `;` (the names, so a changed tag table cannot
//! move a filter). Unknown names are dropped on load.

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, ContainerAcceptsEvent, Flow, Host, HudBatch, ItemStack, MenuButtonEvent, ModContainer,
    NativeScreen, ScreenDrawEvent, Tag,
};

use crate::oredict::{self, Filter};
use crate::{key, merged, read_energy, shrink, write_int};

const FILTERS: &str = "mekanism_od_filters";
const SCROLL: &str = "mekanism_od_scroll";

pub(super) fn load(nbt: &Compound) -> Vec<Filter> {
    let Some(Tag::String(text)) = nbt.get(&key(nbt, FILTERS)) else { return Vec::new() };
    text.to_string_lossy()
        .split(';')
        .filter_map(|entry| {
            let mut part = entry.split(',');
            let tag = oredict::tag_index(part.next()?)?;
            let output = oredict::members(tag).iter().position(|m| Some(*m) == part.next())?;
            let enabled = part.next()? == "1";
            Some(Filter { tag, output, enabled })
        })
        .take(oredict::MAX_FILTERS)
        .collect()
}

fn save(nbt: &mut Compound, filters: &[Filter]) {
    let text: Vec<String> = filters
        .iter()
        .map(|f| format!("{},{},{}", oredict::tag_name(f.tag), oredict::output_of(f).unwrap_or(""), f.enabled as u8))
        .collect();
    let (k, new) = (key(nbt, FILTERS), Tag::String(text.join(";").as_str().into()));
    if nbt.get(&k) != Some(&new) {
        nbt.insert(k, new);
    }
}

/// Hoppers: the bottom takes the output, every other side fills the input.
pub(super) fn container() -> ModContainer {
    ModContainer {
        slots: 2,
        faces: [vec![oredict::SLOT_OUTPUT], vec![oredict::SLOT_INPUT], vec![oredict::SLOT_INPUT], vec![oredict::SLOT_INPUT], vec![oredict::SLOT_INPUT], vec![oredict::SLOT_INPUT]],
        inputs: vec![oredict::SLOT_INPUT],
        outputs: vec![oredict::SLOT_OUTPUT],
        menu: Some(oredict::MENU.to_owned()),
    }
}

pub(super) fn screen() -> NativeScreen {
    NativeScreen {
        width: oredict::WIDTH,
        height: oredict::HEIGHT,
        slots: oredict::SLOT_POS.iter().map(|&p| Some(p)).collect(),
        inventory_y: oredict::INVENTORY_Y,
        inventory_x: oredict::INVENTORY_X,
        own_labels: true,
        own_background: false,
    }
}

/// The output for `stack`: a filter's tag holds it by the table or by the game's own tag.
fn result(host: &dyn Host, filters: &[Filter], stack: &ItemStack) -> Option<&'static str> {
    let mut has = |tag: usize| {
        oredict::members(tag).contains(&&*stack.item)
            || host.server_item_in_tag(stack, oredict::tag_name(tag)).unwrap_or(false)
    };
    oredict::result(filters, &mut has)
}

/// A hopper puts in only what a filter works on.
pub(super) fn accepts(host: &dyn Host, event: &ContainerAcceptsEvent<'_>) -> Flow {
    if event.slot != oredict::SLOT_INPUT {
        return Flow::Continue;
    }
    if result(host, &load(&event.entity.nbt), event.stack).is_some() { Flow::Continue } else { Flow::Cancel }
}

/// `TileEntityOredictionificator.onUpdateServer`: all the input the output has room for,
/// turned into the result, in one tick.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
    let filters = load(&event.entity.nbt);
    let mut items = event.entity.items.clone();
    items.resize(2, None);
    let mut did = false;
    if let Some(input) = items[oredict::SLOT_INPUT].clone() {
        let wanted = result(host, &filters, &input);
        let made = wanted.and_then(|id| host.item_from_snbt(&format!("{{id:\"{id}\",count:1}}")));
        if let Some(made) = made {
            let max = host.max_stack_size(&made.item).unwrap_or(64);
            let have = items[oredict::SLOT_OUTPUT].as_ref();
            let room = match have {
                None => max,
                Some(h) if h.item == made.item && h.extra == made.extra => max.saturating_sub(h.count),
                Some(_) => 0,
            };
            let n = input.count.min(room);
            if n > 0 {
                let out = merged(host, have, &ItemStack { count: n, ..made });
                if out.is_some() {
                    items[oredict::SLOT_OUTPUT] = out;
                    items[oredict::SLOT_INPUT] = shrink(&input, n);
                    did = true;
                }
            }
        }
    }
    let scroll = oredict::clamp_scroll(read_energy(&event.entity.nbt, SCROLL).max(0) as usize, filters.len());
    let e = &mut *event.entity;
    if e.items != items {
        e.items = items;
    }
    let data = oredict::menu_data(&filters, scroll, did);
    if e.data != data {
        e.data = data;
    }
}

pub(super) fn button(event: &mut MenuButtonEvent) {
    let Some(action) = oredict::action_of(event.id) else { return };
    let e = &mut *event.entity;
    let mut filters = load(&e.nbt);
    let mut scroll = read_energy(&e.nbt, SCROLL).max(0) as usize;
    // A new filter starts on the first tag of the item in the input slot.
    let suggest = e
        .items
        .get(oredict::SLOT_INPUT)
        .and_then(|s| s.as_ref())
        .and_then(|s| oredict::tags_of(&s.item).next());
    if !oredict::apply(&mut filters, &mut scroll, action, suggest) {
        return;
    }
    save(&mut e.nbt, &filters);
    write_int(&mut e.nbt, SCROLL, scroll as i32);
    let did = e.data.get(oredict::DATA_DID).copied().unwrap_or(0) != 0;
    e.data = oredict::menu_data(&filters, scroll, did);
}

pub(super) fn click(at: [f32; 2], right: bool) -> Option<i32> {
    oredict::button_at(at, right)
}

/// Title, three filter rows (tag, output, on/off, delete), "New filter" and the progress arrow.
pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let batch = &mut *event.batch;
    let title = host.screen_title().unwrap_or_else(|| "Oredictionificator".to_owned());
    host.text(batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let at = |[rx, ry, w, h]: [f32; 4]| [x + rx, y + ry, w, h];
    let button = |batch: &mut HudBatch, rect: [f32; 4], color: u32, label: &str| {
        let [bx, by, bw, bh] = at(rect);
        host.fill(batch, [bx, by, bw, bh], 0xFF262A2F);
        host.fill(batch, [bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], color);
        host.text(batch, label, [bx + 4.0, by + (bh - 8.0) / 2.0], 0xFFFFFF, true);
    };
    for row in 0..oredict::VISIBLE {
        let Some((tag, out, on)) = oredict::row_texts(event.data, row) else {
            host.fill(batch, at(oredict::row_rect(row, [10.0, 202.0])), 0xFF3B4045);
            continue;
        };
        button(batch, oredict::row_rect(row, oredict::TAG_BUTTON), 0xFF4A6FA5, &oredict::short(tag, 15));
        button(batch, oredict::row_rect(row, oredict::OUT_BUTTON), 0xFF5A7F4F, &oredict::short(out, 10));
        button(batch, oredict::row_rect(row, oredict::ON_BUTTON), if on { 0xFF278447 } else { 0xFF6B7075 }, if on { "On" } else { "--" });
        button(batch, oredict::row_rect(row, oredict::DEL_BUTTON), 0xFFAF4141, "x");
    }
    button(batch, oredict::NEW_BUTTON, 0xFF4A6FA5, "New filter");
    let bar = at(oredict::SCROLL_BAR);
    host.fill(batch, bar, 0xFF242A2F);
    let total = (0..oredict::MAX_FILTERS)
        .filter(|i| event.data.get(oredict::DATA_FILTERS + i * oredict::PER_FILTER).copied().unwrap_or(0) > 0)
        .count();
    if total > oredict::VISIBLE {
        let scroll = event.data.get(oredict::DATA_SCROLL).copied().unwrap_or(0).max(0) as f32;
        let (h, span) = (bar[3] - 2.0, (total - oredict::VISIBLE) as f32);
        let knob = h * oredict::VISIBLE as f32 / total as f32;
        host.fill(batch, [bar[0] + 1.0, bar[1] + 1.0 + (h - knob) * scroll / span, bar[2] - 2.0, knob], 0xFF9AA0A6);
    }
    // The arrow between the slots lights up while it converts.
    let arrow = at([82.0, 118.0, 76.0, 10.0]);
    host.fill(batch, arrow, 0xFF242A2F);
    if event.data.get(oredict::DATA_DID).copied().unwrap_or(0) != 0 {
        host.fill(batch, [arrow[0] + 1.0, arrow[1] + 1.0, arrow[2] - 2.0, arrow[3] - 2.0], 0xFF3FA9F5);
    }
    host.text(batch, "Left: next  Right: previous", [x + 8.0, y + 134.0], 0x555555, false);
    host.text(batch, "Inventory", [x + oredict::INVENTORY_X, y + oredict::INVENTORY_Y - 10.0], 0x404040, false);
}
