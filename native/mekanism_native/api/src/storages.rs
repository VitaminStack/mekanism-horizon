//! The host side of the Personal Chest and Personal Barrel (M08): the 9 x 6
//! container, the owner kept in the block-entity NBT, and the checks that keep
//! other players out. Rules and values are Horizon-free in `storage.rs`.
//!
//! Horizon has no per-player security: the owner is the placer's UUID
//! (`mekanism_owner`) and the checks run in `right_click_block` and
//! `destroy_block`. The container reaches no side, so hoppers cannot see it.

use horizon_client::mod_api::{Compound, Host, ItemStack, ModContainer, Tag, Text};

use crate::storage;
use crate::key;

const OWNER: &str = "mekanism_owner";
const OWNER_NAME: &str = "mekanism_owner_name";

pub(super) fn container() -> ModContainer {
    ModContainer {
        slots: storage::SLOTS,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(storage::MENU.to_owned()),
    }
}

fn owner(nbt: &Compound) -> Option<(u128, String)> {
    let Some(Tag::String(id)) = nbt.get(&key(nbt, OWNER)) else { return None };
    let id = storage::uuid_from_hex(&id.to_string_lossy())?;
    let name = match nbt.get(&key(nbt, OWNER_NAME)) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    Some((id, name))
}

/// The placer becomes the owner.
pub(super) fn place(host: &mut dyn Host, pos: [i32; 3], _stack: &ItemStack) {
    let Some(player) = host.acting_player() else { return };
    host.change_mod_block_entity(pos, &mut |m| {
        if !storage::is_storage(&m.id) {
            return;
        }
        let (id, name) = (key(&m.nbt, OWNER), key(&m.nbt, OWNER_NAME));
        m.nbt.insert(id, Tag::String(storage::uuid_to_hex(player.uuid).as_str().into()));
        m.nbt.insert(name, Tag::String(player.name.as_str().into()));
    });
}

/// Whether the acting player may use the block at `pos`; tells them when not.
pub(super) fn allowed(host: &mut dyn Host, pos: [i32; 3]) -> bool {
    let Some(entity) = host.mod_block_entity(pos) else { return true };
    let Some(player) = host.acting_player() else { return true };
    let own = owner(&entity.nbt);
    if storage::may_use(own.as_ref().map(|o| o.0), player.uuid, player.permission) {
        return true;
    }
    let name = own.map_or_else(String::new, |o| o.1);
    host.action_bar(player.uuid, Text::literal(format!("This is private storage of {name}.")));
    false
}
