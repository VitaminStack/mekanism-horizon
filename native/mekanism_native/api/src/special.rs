//! The special machines of M08, one `Kind::Special(Block)` in `lib.rs`: this module finds
//! the block and hands each hook to the block's own module (`oredicts.rs`, `miners.rs`), so
//! `lib.rs` stays one arm per hook.

use horizon_client::mod_api::{
    BlockEntityTickEvent, ContainerAcceptsEvent, Flow, Host, MenuButtonEvent, ModContainer, ModEnergyBuffer,
    NativeScreen, ScreenDrawEvent,
};

use crate::{miner, miners, oredicts};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Oredictionificator,
    DigitalMiner,
}

pub const OREDICTIONIFICATOR: &str = "mekanism:oredictionificator";

pub fn find(block: &str) -> Option<Block> {
    match block {
        OREDICTIONIFICATOR => Some(Block::Oredictionificator),
        miner::BLOCK => Some(Block::DigitalMiner),
        _ => None,
    }
}

pub fn container(b: Block) -> ModContainer {
    match b {
        Block::Oredictionificator => oredicts::container(),
        Block::DigitalMiner => miners::container(),
    }
}

/// Only the miner holds energy (the Oredictionificator uses none).
pub fn energy_buffer(b: Block) -> Option<ModEnergyBuffer> {
    match b {
        Block::Oredictionificator => None,
        Block::DigitalMiner => Some(miners::energy_buffer()),
    }
}

pub fn screen(b: Block) -> NativeScreen {
    match b {
        Block::Oredictionificator => oredicts::screen(),
        Block::DigitalMiner => miners::screen(),
    }
}

pub fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, b: Block) {
    match b {
        Block::Oredictionificator => oredicts::draw(host, event),
        Block::DigitalMiner => miners::draw(host, event),
    }
}

pub fn click(b: Block, at: [f32; 2], right: bool) -> Option<i32> {
    match b {
        Block::Oredictionificator => oredicts::click(at, right),
        Block::DigitalMiner => miners::click(at),
    }
}

pub fn button(host: &dyn Host, event: &mut MenuButtonEvent, b: Block) {
    match b {
        Block::Oredictionificator => oredicts::button(event),
        Block::DigitalMiner => miners::button(host, event),
    }
}

pub fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, b: Block) {
    match b {
        Block::Oredictionificator => oredicts::tick(host, event),
        Block::DigitalMiner => miners::tick(host, event),
    }
}

pub fn accepts(host: &dyn Host, event: &ContainerAcceptsEvent<'_>, b: Block) -> Flow {
    match b {
        Block::Oredictionificator => oredicts::accepts(host, event),
        Block::DigitalMiner => Flow::Continue,
    }
}

/// A placed multiblock main block gets its bounding blocks.
pub fn place_block(host: &mut dyn Host, item: &str, pos: [i32; 3]) -> bool {
    if item != miner::BLOCK {
        return false;
    }
    miners::place(host, pos);
    true
}

/// Breaking a multiblock's main or bounding block; whether it was one of M08's.
pub fn destroy_block(host: &mut dyn Host, pos: [i32; 3]) -> bool {
    miners::destroy(host, pos)
}

/// A bounding block of a miner has no screen of its own (the generators' bounding blocks do).
pub fn is_miner_bounding(host: &dyn Host, pos: [i32; 3]) -> bool {
    miners::main_of(host, pos).is_some()
}
