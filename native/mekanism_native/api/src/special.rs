//! The special machines of M08, one `Kind::Special(Block)` in `lib.rs`: this module finds
//! the block and hands each hook to the block's own module (`oredicts.rs`, later
//! `assemblicators.rs` and `miners.rs`), so `lib.rs` stays one arm per hook.

use horizon_client::mod_api::{
    BlockEntityTickEvent, ContainerAcceptsEvent, Flow, Host, MenuButtonEvent, ModContainer, NativeScreen,
    ScreenDrawEvent,
};

use crate::oredicts;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Oredictionificator,
}

pub const OREDICTIONIFICATOR: &str = "mekanism:oredictionificator";

pub fn find(block: &str) -> Option<Block> {
    match block {
        OREDICTIONIFICATOR => Some(Block::Oredictionificator),
        _ => None,
    }
}

pub fn container(b: Block) -> ModContainer {
    match b {
        Block::Oredictionificator => oredicts::container(),
    }
}

pub fn screen(b: Block) -> NativeScreen {
    match b {
        Block::Oredictionificator => oredicts::screen(),
    }
}

pub fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, b: Block) {
    match b {
        Block::Oredictionificator => oredicts::draw(host, event),
    }
}

pub fn click(b: Block, at: [f32; 2], right: bool) -> Option<i32> {
    match b {
        Block::Oredictionificator => oredicts::click(at, right),
    }
}

pub fn button(b: Block, event: &mut MenuButtonEvent) {
    match b {
        Block::Oredictionificator => oredicts::button(event),
    }
}

pub fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, b: Block) {
    match b {
        Block::Oredictionificator => oredicts::tick(host, event),
    }
}

pub fn accepts(host: &dyn Host, event: &ContainerAcceptsEvent<'_>, b: Block) -> Flow {
    match b {
        Block::Oredictionificator => oredicts::accepts(host, event),
    }
}
