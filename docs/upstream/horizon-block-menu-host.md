# Draft upstream request: open a block's menu from another block

Not posted. Draft for the Horizon repository (pinned `fdc87ae`, API 411).

## Problem

Mekanism's multiblocks (Wind Generator, Advanced Solar Generator) fill their
shape with bounding blocks (`mekanism:bounding_block`). In Mekanism a right
click on any bounding block opens the main block's screen (`BlockBounding`
forwards `useWithoutItem` to the main block).

A native trait mod cannot do that: the `Host` trait has no call that opens a
block entity's menu (`ModContainer::menu`) for a player at a given position,
and `use_item_on` fires only with an item in hand. So the screen opens only
on the main (bottom) block.

## Request

- A hook for using a block without an item (`useWithoutItem`), claimable.
- `Host::open_block_menu(player, pos) -> bool`: opens the menu the block
  entity at `pos` declares, as a right click on that block would.
