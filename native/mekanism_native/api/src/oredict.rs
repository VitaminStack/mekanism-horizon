//! The Oredictionificator's rules (M08), Horizon-free: its filters, the choice of result,
//! the screen layout and the menu data. The host side is `oredicts.rs`.
//!
//! Upstream: `TileEntityOredictionificator` (no energy; two slots, input (56, 115) and
//! output (164, 115); redstone only gates it), `OredictionificatorItemFilter` (a tag and a
//! selected output), `GeneralConfig` (`validOredictionificatorFilters`). Horizon can only
//! test an item against a tag, not list a tag, so the tags and their members come from the
//! generated `oredict_tags.rs`; a filter is the indices into that table.

use crate::oredict_tags::TAGS;

pub const MENU: &str = "mekanism:oredictionificator";
pub const SLOT_INPUT: usize = 0;
pub const SLOT_OUTPUT: usize = 1;
/// The most filters one block keeps (upstream has no limit; the menu data is a fixed size).
pub const MAX_FILTERS: usize = 8;
/// `GuiOredictionificator.FILTER_COUNT`: filters shown at once.
pub const VISIBLE: usize = 3;

pub const DATA_DID: usize = 0;
pub const DATA_SCROLL: usize = 1;
pub const DATA_FILTERS: usize = 2;
/// Per filter: tag index + 1 (0 for an empty row), output index, enabled.
pub const PER_FILTER: usize = 3;
pub const DATA_LEN: usize = DATA_FILTERS + MAX_FILTERS * PER_FILTER;

/// `GuiOredictionificator`: 176 + 60 by 166 + 64; the inventory is offset by (30, 64).
pub const WIDTH: f32 = 236.0;
pub const HEIGHT: f32 = 230.0;
pub const INVENTORY_X: f32 = 38.0;
pub const INVENTORY_Y: f32 = 148.0;
pub const SLOT_POS: [[f32; 2]; 2] = [[56.0, 115.0], [164.0, 115.0]];

/// A filter: the tag to match, which of its members comes out, and whether it works.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Filter {
    pub tag: usize,
    pub output: usize,
    pub enabled: bool,
}

pub fn tag_name(tag: usize) -> &'static str {
    TAGS.get(tag).map_or("", |t| t.0)
}

pub fn tag_index(name: &str) -> Option<usize> {
    TAGS.iter().position(|t| t.0 == name)
}

pub fn members(tag: usize) -> &'static [&'static str] {
    TAGS.get(tag).map_or(&[], |t| t.1)
}

/// The item a filter turns things into (`filter.getResult()`).
pub fn output_of(f: &Filter) -> Option<&'static str> {
    members(f.tag).get(f.output).copied()
}

/// Every table tag whose members hold `item`, in table order.
pub fn tags_of(item: &str) -> impl Iterator<Item = usize> + '_ {
    TAGS.iter().enumerate().filter(move |(_, t)| t.1.contains(&item)).map(|(i, _)| i)
}

/// Upstream `getResult`: the output of the first enabled filter whose tag holds the input.
/// `has_tag` answers for the tag with this index (the table, and the game's own tags).
pub fn result(filters: &[Filter], has_tag: &mut dyn FnMut(usize) -> bool) -> Option<&'static str> {
    filters.iter().filter(|f| f.enabled).find(|f| has_tag(f.tag)).and_then(output_of)
}

/// `OredictionificatorFilter.next`/`previous` for the output: wraps round the tag's members.
pub fn cycle_output(f: &mut Filter, forward: bool) {
    let n = members(f.tag).len().max(1);
    f.output = if forward { (f.output + 1) % n } else { (f.output + n - 1) % n };
}

/// Another tag (wrapping). As upstream's `setFilter`, the output stays if the new tag holds
/// it; else it is the new tag's first member.
pub fn cycle_tag(f: &mut Filter, forward: bool) {
    let kept = output_of(f);
    let n = TAGS.len();
    f.tag = if forward { (f.tag + 1) % n } else { (f.tag + n - 1) % n };
    f.output = kept.and_then(|k| members(f.tag).iter().position(|m| *m == k)).unwrap_or(0);
}

/// What a click on the screen does. Ids are `kind * 16 + row` (row is the visible row).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Tag { row: usize, forward: bool },
    Output { row: usize, forward: bool },
    Toggle { row: usize },
    Delete { row: usize },
    New,
    Scroll { down: bool },
}

const K_TAG: i32 = 1;
const K_OUT: i32 = 3;
const K_TOGGLE: i32 = 5;
const K_DELETE: i32 = 6;
const K_NEW: i32 = 7;
const K_SCROLL: i32 = 8;

pub fn id_of(a: Action) -> i32 {
    let (kind, row) = match a {
        Action::Tag { row, forward } => (K_TAG + !forward as i32, row),
        Action::Output { row, forward } => (K_OUT + !forward as i32, row),
        Action::Toggle { row } => (K_TOGGLE, row),
        Action::Delete { row } => (K_DELETE, row),
        Action::New => (K_NEW, 0),
        Action::Scroll { down } => (K_SCROLL, down as usize),
    };
    kind * 16 + row as i32
}

pub fn action_of(id: i32) -> Option<Action> {
    let (kind, row) = (id.div_euclid(16), id.rem_euclid(16) as usize);
    let in_rows = row < VISIBLE;
    Some(match kind {
        K_TAG | 2 if in_rows => Action::Tag { row, forward: kind == K_TAG },
        K_OUT | 4 if in_rows => Action::Output { row, forward: kind == K_OUT },
        K_TOGGLE if in_rows => Action::Toggle { row },
        K_DELETE if in_rows => Action::Delete { row },
        K_NEW => Action::New,
        K_SCROLL if row < 2 => Action::Scroll { down: row == 1 },
        _ => return None,
    })
}

/// Rows of `GuiOredictionificator`: a filter's tag button, output button, on/off switch and
/// delete button, the "new filter" button below them and the scroll bar at the right.
pub const ROW_Y: f32 = 16.0;
pub const ROW_STEP: f32 = 24.0;
pub const ROW_H: f32 = 22.0;
pub const TAG_BUTTON: [f32; 2] = [10.0, 100.0];
pub const OUT_BUTTON: [f32; 2] = [112.0, 66.0];
pub const ON_BUTTON: [f32; 2] = [180.0, 18.0];
pub const DEL_BUTTON: [f32; 2] = [200.0, 12.0];
pub const NEW_BUTTON: [f32; 4] = [10.0, 90.0, 202.0, 20.0];
pub const SCROLL_BAR: [f32; 4] = [213.0, 17.0, 6.0, 90.0];

pub fn row_rect(row: usize, [x, w]: [f32; 2]) -> [f32; 4] {
    [x, ROW_Y + ROW_STEP * row as f32, w, ROW_H]
}

fn inside([x, y, w, h]: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

/// The button id under `at` (relative to the screen's corner); the right button goes back.
pub fn button_at(at: [f32; 2], right: bool) -> Option<i32> {
    if inside(NEW_BUTTON, at) {
        return Some(id_of(Action::New));
    }
    if inside(SCROLL_BAR, at) {
        return Some(id_of(Action::Scroll { down: at[1] >= SCROLL_BAR[1] + SCROLL_BAR[3] / 2.0 }));
    }
    (0..VISIBLE).find_map(|row| {
        let hit = |b: [f32; 2]| inside(row_rect(row, b), at);
        let action = if hit(TAG_BUTTON) {
            Action::Tag { row, forward: !right }
        } else if hit(OUT_BUTTON) {
            Action::Output { row, forward: !right }
        } else if hit(ON_BUTTON) {
            Action::Toggle { row }
        } else if hit(DEL_BUTTON) {
            Action::Delete { row }
        } else {
            return None;
        };
        Some(id_of(action))
    })
}

/// A press: edits the filters and scroll. `suggest` is the tag a new filter starts with (the
/// input item's first one, else the table's first). Returns whether anything changed.
pub fn apply(filters: &mut Vec<Filter>, scroll: &mut usize, action: Action, suggest: Option<usize>) -> bool {
    let row_filter = |filters: &mut Vec<Filter>, row: usize| {
        let i = *scroll + row;
        (i < filters.len()).then_some(i)
    };
    match action {
        Action::New => {
            if filters.len() >= MAX_FILTERS {
                return false;
            }
            filters.push(Filter { tag: suggest.unwrap_or(0), output: 0, enabled: true });
            *scroll = clamp_scroll(*scroll, filters.len());
        }
        Action::Scroll { down } => {
            let next = if down { *scroll + 1 } else { scroll.saturating_sub(1) };
            let next = clamp_scroll(next, filters.len());
            if next == *scroll {
                return false;
            }
            *scroll = next;
        }
        Action::Delete { row } => {
            let Some(i) = row_filter(filters, row) else { return false };
            filters.remove(i);
            *scroll = clamp_scroll(*scroll, filters.len());
        }
        Action::Toggle { row } => {
            let Some(i) = row_filter(filters, row) else { return false };
            filters[i].enabled = !filters[i].enabled;
        }
        Action::Tag { row, forward } => {
            let Some(i) = row_filter(filters, row) else { return false };
            cycle_tag(&mut filters[i], forward);
        }
        Action::Output { row, forward } => {
            let Some(i) = row_filter(filters, row) else { return false };
            cycle_output(&mut filters[i], forward);
        }
    }
    true
}

pub fn clamp_scroll(scroll: usize, count: usize) -> usize {
    scroll.min(count.saturating_sub(VISIBLE))
}

/// The menu data: `did_process`, scroll, then each filter's three ints (zeros for none).
pub fn menu_data(filters: &[Filter], scroll: usize, did: bool) -> Vec<i32> {
    let mut data = vec![0; DATA_LEN];
    data[DATA_DID] = did as i32;
    data[DATA_SCROLL] = scroll as i32;
    for (i, f) in filters.iter().take(MAX_FILTERS).enumerate() {
        let at = DATA_FILTERS + i * PER_FILTER;
        data[at] = f.tag as i32 + 1;
        data[at + 1] = f.output as i32;
        data[at + 2] = f.enabled as i32;
    }
    data
}

/// The filter in row `row` (after the scroll) as the screen shows it: tag name, output name.
pub fn row_texts(data: &[i32], row: usize) -> Option<(&'static str, &'static str, bool)> {
    let scroll = data.get(DATA_SCROLL).copied().unwrap_or(0).max(0) as usize;
    let at = DATA_FILTERS + (scroll + row) * PER_FILTER;
    let tag = (*data.get(at)? as usize).checked_sub(1)?;
    let out = members(tag).get(*data.get(at + 1)? as usize).copied().unwrap_or("");
    Some((tag_name(tag), out, data.get(at + 2).copied().unwrap_or(0) != 0))
}

/// `minecraft:iron_ingot` as `iron_ingot`; `c:ingots/iron` as `ingots/iron`; cut to `max` chars.
pub fn short(name: &str, max: usize) -> String {
    let path = name.split_once(':').map_or(name, |(_, p)| p);
    path.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iron() -> usize {
        tag_index("c:ingots/iron").expect("table has c:ingots/iron")
    }

    #[test]
    fn table_holds_only_valid_prefixes() {
        for (tag, items) in TAGS.iter() {
            let path = tag.strip_prefix("c:").expect("namespace c");
            assert!(
                ["ingots/", "ores/", "dusts/", "nuggets/", "storage_blocks/", "raw_materials/"]
                    .iter()
                    .any(|p| path.starts_with(p)),
                "{tag}"
            );
            assert!(!items.is_empty(), "{tag}");
        }
    }

    #[test]
    fn result_is_the_first_enabled_matching_filter() {
        let f = [
            Filter { tag: iron(), output: 0, enabled: false },
            Filter { tag: iron(), output: 0, enabled: true },
        ];
        let want = tag_index("c:ingots/iron");
        let got = result(&f, &mut |t| Some(t) == want);
        assert_eq!(got, members(iron()).first().copied());
        // Only the disabled filter: nothing comes out.
        assert_eq!(result(&f[..1], &mut |_| true), None);
        // The tag does not hold the input.
        assert_eq!(result(&f, &mut |_| false), None);
    }

    #[test]
    fn output_cycling_wraps_like_upstream() {
        let multi = TAGS.iter().position(|t| t.1.len() > 1).expect("a tag with several items");
        let n = members(multi).len();
        let mut f = Filter { tag: multi, output: 0, enabled: true };
        cycle_output(&mut f, false);
        assert_eq!(f.output, n - 1);
        cycle_output(&mut f, true);
        assert_eq!(f.output, 0);
        for _ in 0..n {
            cycle_output(&mut f, true);
        }
        assert_eq!(f.output, 0);
    }

    #[test]
    fn changing_the_tag_keeps_a_member_that_both_hold() {
        // An item in two tags: the output stays when the filter moves between them.
        let item = TAGS.iter().flat_map(|t| t.1.iter()).find(|i| tags_of(i).count() > 1);
        let Some(item) = item else { return };
        let tags: Vec<usize> = tags_of(item).collect();
        let pos = members(tags[0]).iter().position(|m| m == item).unwrap();
        let mut f = Filter { tag: tags[0], output: pos, enabled: true };
        // Walk to the next tag holding the same item.
        while f.tag != tags[1] {
            cycle_tag(&mut f, true);
        }
        assert_eq!(output_of(&f), Some(*item));
        // A tag without it resets to the first member.
        let other = (0..TAGS.len()).find(|t| !members(*t).contains(item)).unwrap();
        let mut g = Filter { tag: tags[0], output: pos, enabled: true };
        g.tag = other;
        cycle_tag(&mut g, false);
        assert!(g.output < members(g.tag).len());
    }

    #[test]
    fn filters_are_edited_by_rows_and_scroll() {
        let mut filters = Vec::new();
        let mut scroll = 0;
        for _ in 0..MAX_FILTERS {
            assert!(apply(&mut filters, &mut scroll, Action::New, Some(iron())));
        }
        assert!(!apply(&mut filters, &mut scroll, Action::New, None), "full");
        assert_eq!(scroll, 0, "a new filter does not scroll");
        scroll = MAX_FILTERS - VISIBLE;
        // Row 0 is now filter 5; deleting it keeps the scroll in range.
        assert!(apply(&mut filters, &mut scroll, Action::Toggle { row: 0 }, None));
        assert!(!filters[MAX_FILTERS - VISIBLE].enabled);
        assert!(apply(&mut filters, &mut scroll, Action::Delete { row: 0 }, None));
        assert_eq!(filters.len(), MAX_FILTERS - 1);
        assert_eq!(scroll, filters.len() - VISIBLE);
        assert!(!apply(&mut filters, &mut scroll, Action::Scroll { down: true }, None));
        assert!(apply(&mut filters, &mut scroll, Action::Scroll { down: false }, None));
        // A row past the filters does nothing.
        let mut few = vec![Filter { tag: 0, output: 0, enabled: true }];
        let mut s = 0;
        assert!(!apply(&mut few, &mut s, Action::Delete { row: 2 }, None));
    }

    #[test]
    fn buttons_round_trip_and_hit_the_layout() {
        for row in 0..VISIBLE {
            for forward in [true, false] {
                for a in [Action::Tag { row, forward }, Action::Output { row, forward }] {
                    assert_eq!(action_of(id_of(a)), Some(a));
                }
            }
            for a in [Action::Toggle { row }, Action::Delete { row }] {
                assert_eq!(action_of(id_of(a)), Some(a));
            }
        }
        assert_eq!(action_of(id_of(Action::New)), Some(Action::New));
        assert_eq!(action_of(id_of(Action::Scroll { down: true })), Some(Action::Scroll { down: true }));
        assert_eq!(action_of(K_TAG * 16 + 5), None, "row outside the screen");
        let mid = |r: [f32; 4]| [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0];
        let hit = |at, right| button_at(at, right).and_then(action_of);
        assert_eq!(hit(mid(row_rect(1, TAG_BUTTON)), false), Some(Action::Tag { row: 1, forward: true }));
        assert_eq!(hit(mid(row_rect(2, OUT_BUTTON)), true), Some(Action::Output { row: 2, forward: false }));
        assert_eq!(hit(mid(row_rect(0, ON_BUTTON)), false), Some(Action::Toggle { row: 0 }));
        assert_eq!(hit(mid(row_rect(0, DEL_BUTTON)), false), Some(Action::Delete { row: 0 }));
        assert_eq!(hit(mid(NEW_BUTTON), false), Some(Action::New));
        assert_eq!(hit([214.0, 20.0], false), Some(Action::Scroll { down: false }));
        assert_eq!(hit([214.0, 100.0], false), Some(Action::Scroll { down: true }));
        assert_eq!(hit([100.0, 160.0], false), None);
        // The new button and the slots do not overlap.
        assert!(NEW_BUTTON[1] + NEW_BUTTON[3] <= SLOT_POS[0][1]);
        assert!(row_rect(VISIBLE - 1, TAG_BUTTON)[1] + ROW_H <= NEW_BUTTON[1]);
    }

    #[test]
    fn menu_data_carries_the_visible_rows() {
        let filters: Vec<Filter> = (0..5).map(|i| Filter { tag: i, output: 0, enabled: i % 2 == 0 }).collect();
        let data = menu_data(&filters, 2, true);
        assert_eq!(data.len(), DATA_LEN);
        assert_eq!(data[DATA_DID], 1);
        let (tag, out, on) = row_texts(&data, 0).unwrap();
        assert_eq!(tag, tag_name(2));
        assert_eq!(out, members(2)[0]);
        assert!(on);
        assert!(row_texts(&data, 3).is_none(), "past the last filter");
        assert_eq!(short("minecraft:iron_ingot", 20), "iron_ingot");
        assert_eq!(short("c:storage_blocks/raw_iron", 8), "storage_");
    }
}
