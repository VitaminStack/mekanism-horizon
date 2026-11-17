//! The host side of Mekanism's multiblocks (M09): formation, links between the
//! members, re-validation and the shared state's owner. The Horizon-free
//! validation is `multiblock.rs`; each structure's own rules and ticks are in
//! its module (`dynamic_tanks.rs`).
//!
//! Every shell block with a block entity (casing, valve, port, controller) stores
//! `mekanism_mb_master`, the position of the structure's master. The master is the
//! controller, else the lowest casing (`Structure::master`), and holds the shared
//! state (and the bounds, `mekanism_mb_min`/`max`). Nothing here uses neighbour or
//! chunk events; the members poll in their ticks, staggered by position:
//! - an unformed member tries to form every 200 ticks (and at once when placed);
//! - a formed member checks its master every 10 ticks, and dissolves itself when
//!   the master is gone or no longer claims it;
//! - the master re-validates the whole structure every 20 ticks and dissolves
//!   when it is broken. Chunks that are not loaded count as "unknown": nothing
//!   forms or dissolves because of them.
//! A dissolved master keeps its contents as a cache; the next structure that
//! forms around it adds up the caches of all its members (`multiblock::merge_stored`).
//! Breaking the master by hand passes its contents to a neighbouring member first.

use horizon_client::mod_api::{BlockEntityTickEvent, Compound, Host, Tag};

use super::multiblock::{self, Cell, Def, Outcome, Pos, Structure, Type};
use super::{dynamic_tank, dynamic_tanks, key, offset};

const MASTER: &str = "mekanism_mb_master";
const MIN: &str = "mekanism_mb_min";
const MAX: &str = "mekanism_mb_max";

const DEFS: &[&Def] = &[&dynamic_tank::DEF];

/// The structure type `block` is a member (with a block entity) of.
pub(super) fn find(block: &str) -> Option<&'static Def> {
    DEFS.iter().copied().find(|d| d.is_member(block))
}

pub(super) fn read_pos(nbt: &Compound, name: &'static str) -> Option<Pos> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::IntArray(v)) if v.len() == 3 => Some([v[0], v[1], v[2]]),
        _ => None,
    }
}

fn write_pos(nbt: &mut Compound, name: &'static str, p: Pos) {
    let k = key(nbt, name);
    let tag = Tag::IntArray(p.to_vec());
    if nbt.get(&k) != Some(&tag) {
        nbt.insert(k, tag);
    }
}

fn remove(nbt: &mut Compound, name: &'static str) {
    let k = key(nbt, name);
    nbt.remove(&k);
}

/// The master's bounds, if `pos` lies inside them.
fn claims(nbt: &Compound, pos: Pos) -> bool {
    match (read_pos(nbt, MIN), read_pos(nbt, MAX)) {
        (Some(lo), Some(hi)) => (0..3).all(|a| pos[a] >= lo[a] && pos[a] <= hi[a]),
        _ => false,
    }
}

fn set_formed(host: &mut dyn Host, pos: Pos, on: bool) {
    let want = if on { "true" } else { "false" };
    if host.block_property(pos, "formed").as_deref() != Some(want) {
        host.set_block_property(pos, "formed", want);
    }
}

fn is_loaded(host: &mut dyn Host, pos: Pos) -> bool {
    let dimension = host.dimension().to_owned();
    host.level(&dimension).is_some_and(|l| l.is_loaded(&*host, pos))
}

fn game_time(host: &mut dyn Host) -> i64 {
    let dimension = host.dimension().to_owned();
    host.level(&dimension).and_then(|l| l.game_time(&*host)).unwrap_or(0)
}

/// Validate the structure `seed` is part of, reading the world through `host`.
fn validate(host: &mut dyn Host, def: &Def, seed: Pos) -> Outcome {
    let dimension = host.dimension().to_owned();
    let Some(level) = host.level(&dimension) else { return Outcome::Unloaded };
    let host: &dyn Host = &*host;
    multiblock::validate(seed, &def.rules, &mut |p| {
        if !level.is_loaded(host, p) { Cell::Unloaded } else { def.classify(&host.block(p)) }
    })
}

/// One member's link to `master`; the master also gets the bounds and the merged contents.
fn link(nbt: &mut Compound, def: &Def, s: &Structure, master: Pos, here: Pos, merged: &Compound) {
    write_pos(nbt, MASTER, master);
    if here == master {
        write_pos(nbt, MIN, s.min);
        write_pos(nbt, MAX, s.max);
        for (k, v) in merged.iter() {
            if nbt.get(k) != Some(v) {
                nbt.insert(k.clone(), v.clone());
            }
        }
        if let Type::Tank = def.ty {
            dynamic_tanks::validated(nbt, s);
        }
    } else {
        remove(nbt, MIN);
        remove(nbt, MAX);
        if let Type::Tank = def.ty {
            dynamic_tanks::clear(nbt);
        }
    }
}

/// Link all members of `s` to its master and give it their merged contents. `own` is the
/// block entity whose tick is running (its changes must go through the tick's own copy).
fn form(host: &mut dyn Host, def: &Def, s: &Structure, mut own: Option<(Pos, &mut Compound)>) {
    let members: Vec<Pos> = s.shell.iter().copied().filter(|&p| def.is_member(&host.block(p))).collect();
    let Some(master) = s.master(|p| members.contains(&p)) else { return };
    let parts: Vec<Compound> = members
        .iter()
        .filter_map(|&p| match own.as_ref() {
            Some((at, nbt)) if *at == p => Some((**nbt).clone()),
            _ => host.mod_block_entity(p).map(|e| e.nbt.clone()),
        })
        .collect();
    let merged = match def.ty {
        Type::Tank => dynamic_tanks::merge(&parts),
        Type::Matrix | Type::Evaporation => Compound::new(),
    };
    for &p in &members {
        match own.as_mut() {
            Some((at, nbt)) if *at == p => link(nbt, def, s, master, p, &merged),
            _ => {
                host.change_mod_block_entity(p, &mut |m| link(&mut m.nbt, def, s, master, p, &merged));
            }
        }
        set_formed(host, p, true);
    }
}

/// Make `nbt`'s block no longer part of a structure (the master keeps its contents).
fn unlink(nbt: &mut Compound) {
    remove(nbt, MASTER);
    remove(nbt, MIN);
    remove(nbt, MAX);
}

/// Try to form the structure `seed` belongs to. Returns whether one formed.
fn try_form(host: &mut dyn Host, def: &Def, seed: Pos, own: Option<(Pos, &mut Compound)>) -> bool {
    match validate(host, def, seed) {
        Outcome::Formed(s) => {
            form(host, def, &s, own);
            true
        }
        _ => false,
    }
}

/// A member block was placed: form at once, joining or completing a structure.
pub(super) fn placed(host: &mut dyn Host, pos: Pos, block: &str) {
    if let Some(def) = find(block) {
        try_form(host, def, pos, None);
    }
}

/// A member block is about to be broken by a player: its contents (only the master has
/// any, or a dissolved master's cache) go to a neighbouring member, which carries them
/// to the next structure formed there.
pub(super) fn broken(host: &mut dyn Host, pos: Pos, block: &str) {
    let Some(def) = find(block) else { return };
    let Some(entity) = host.mod_block_entity(pos) else { return };
    let (stored, nbt) = match def.ty {
        Type::Tank => (dynamic_tanks::has_contents(&entity.nbt), entity.nbt.clone()),
        _ => return,
    };
    if !stored {
        return;
    }
    for dir in 0..6 {
        let at = offset(pos, dir);
        if !def.is_member(&host.block(at)) {
            continue;
        }
        host.change_mod_block_entity(at, &mut |m| {
            if def.is_member(&m.id) {
                dynamic_tanks::inherit(&mut m.nbt, &nbt);
            }
        });
        break;
    }
}

const RETRY: i64 = 200;
const LINK_CHECK: i64 = 10;
const MASTER_CHECK: i64 = 20;

fn stagger(pos: Pos) -> i64 {
    (pos[0] as i64).wrapping_mul(31).wrapping_add((pos[1] as i64).wrapping_mul(17)).wrapping_add((pos[2] as i64).wrapping_mul(13))
}

/// One tick of a member block.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, def: &'static Def) {
    let pos = event.pos;
    let slot = (game_time(host) + stagger(pos)).rem_euclid(RETRY);
    match read_pos(&event.entity.nbt, MASTER) {
        None => {
            if slot == 0 {
                if !try_form(host, def, pos, Some((pos, &mut event.entity.nbt))) {
                    set_formed(host, pos, false);
                }
            }
            if !event.entity.data.is_empty() {
                event.entity.data.clear();
            }
        }
        Some(master) if master == pos => master_tick(host, event, def, slot),
        Some(master) => {
            if slot % LINK_CHECK == 0 {
                member_check(host, event, def, master);
            }
            if matches!(def.ty, Type::Tank) {
                dynamic_tanks::member_tick(host, event, master);
            }
        }
    }
}

fn member_check(host: &mut dyn Host, event: &mut BlockEntityTickEvent, def: &Def, master: Pos) {
    if !is_loaded(host, master) {
        return;
    }
    let ok = host
        .mod_block_entity(master)
        .is_some_and(|e| def.is_member(&e.id) && read_pos(&e.nbt, MASTER) == Some(master) && claims(&e.nbt, event.pos));
    if ok {
        if let Some(e) = host.mod_block_entity(master) {
            if matches!(def.ty, Type::Tank) {
                let data = dynamic_tanks::menu_data(&e.nbt, true).to_vec();
                if event.entity.data != data {
                    event.entity.data = data;
                }
            }
        }
    } else {
        unlink(&mut event.entity.nbt);
        event.entity.data.clear();
        set_formed(host, event.pos, false);
    }
}

fn master_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, def: &'static Def, slot: i64) {
    let pos = event.pos;
    if slot % MASTER_CHECK == 0 {
        match validate(host, def, pos) {
            Outcome::Formed(s) if s.min == read_pos(&event.entity.nbt, MIN).unwrap_or(s.min) && s.max == read_pos(&event.entity.nbt, MAX).unwrap_or(s.max)
                && s.master(|p| def.is_member(&host.block(p))) == Some(pos) =>
            {
                if let Type::Tank = def.ty {
                    dynamic_tanks::validated(&mut event.entity.nbt, &s);
                }
            }
            // The shape changed (a block added or moved the master): dissolve; the structure forms
            // again from the block that changed, or by the members' retries.
            Outcome::Formed(_) => {
                unlink(&mut event.entity.nbt);
                set_formed(host, pos, false);
                return;
            }
            Outcome::Invalid(_) => {
                unlink(&mut event.entity.nbt);
                set_formed(host, pos, false);
                return;
            }
            Outcome::Unloaded => {}
        }
    }
    if let Type::Tank = def.ty {
        dynamic_tanks::master_tick(&event.entity.nbt, &mut event.entity.data);
    }
}
