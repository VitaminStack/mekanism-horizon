//! Personal Chest and Personal Barrel (M08), the parts that need no Horizon
//! types, so their tests run in `logic/energy`. The host side is `storages.rs`.
//!
//! Mekanism 26.3 (`TileEntityPersonalStorage`, `PersonalStorageManager`): both
//! blocks have 54 slots (a 9 x 6 grid) and are owned by the player who placed
//! them; their `SecurityMode` defaults to private, so only the owner (and
//! operators) can open or break them and automation (hoppers) is shut out.
//!
//! Not ported: the Security Desk (public/trusted modes, trusted lists) and the
//! storage kept on the item (a broken block drops what Horizon drops from a
//! mod container). Without a Security Desk the blocks are always private.

pub const CHEST: &str = "mekanism:personal_chest";
pub const BARREL: &str = "mekanism:personal_barrel";

/// 9 x 6 (`PersonalStorageManager`).
pub const SLOTS: usize = 54;
/// Horizon's built-in 9 x 6 chest menu (`containers::MOD_MENUS`).
pub const MENU: &str = "minecraft:generic_9x6";

/// The block's own id when it is private storage.
pub fn find(block: &str) -> Option<&'static str> {
    [CHEST, BARREL].into_iter().find(|b| *b == block)
}

pub fn is_storage(block: &str) -> bool {
    block == CHEST || block == BARREL
}

/// A chest only faces the four horizontal directions (its blockstate has no up/down).
pub fn horizontal_only(block: &str) -> bool {
    block == CHEST
}

/// Whether `player` may open or break a block owned by `owner` (none: nobody
/// owns it, e.g. placed by a command, so everyone may). Operators (permission
/// level 2 and up) may always, as Mekanism's admins can.
pub fn may_use(owner: Option<u128>, player: u128, permission: u8) -> bool {
    owner.is_none_or(|o| o == player) || permission >= 2
}

/// `0123abcd...` as a string in NBT: a 128-bit UUID in 32 hex digits.
pub fn uuid_to_hex(id: u128) -> String {
    format!("{id:032x}")
}

pub fn uuid_from_hex(s: &str) -> Option<u128> {
    (s.len() == 32).then(|| u128::from_str_radix(s, 16).ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_and_operators_may_use() {
        assert!(may_use(None, 7, 0), "unowned: everyone");
        assert!(may_use(Some(7), 7, 0), "the owner");
        assert!(!may_use(Some(7), 8, 0), "someone else");
        assert!(!may_use(Some(7), 8, 1), "a level 1 player is no operator here");
        assert!(may_use(Some(7), 8, 2), "an operator");
    }

    #[test]
    fn uuid_round_trips() {
        let id = 0x0123_4567_89ab_cdef_0011_2233_4455_6677u128;
        assert_eq!(uuid_to_hex(id).len(), 32);
        assert_eq!(uuid_from_hex(&uuid_to_hex(id)), Some(id));
        assert_eq!(uuid_from_hex(&uuid_to_hex(5)), Some(5), "leading zeros stay");
        assert_eq!(uuid_from_hex("xyz"), None);
        assert_eq!(uuid_from_hex(&"g".repeat(32)), None);
    }

    #[test]
    fn blocks_are_told_apart() {
        assert!(is_storage(CHEST) && is_storage(BARREL));
        assert!(!is_storage("mekanism:basic_bin"));
        assert!(horizontal_only(CHEST) && !horizontal_only(BARREL));
        assert_eq!(SLOTS, 9 * 6);
    }
}
