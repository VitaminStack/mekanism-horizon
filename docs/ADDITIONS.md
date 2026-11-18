# Mekanism: Additions (M12)

Content mod `content/mekanismadditions` (namespace `mekanismadditions`). Pure content, no native code: every block is a vanilla template (`template`) plus `properties`.

Generated with `python -I scripts/gen_additions.py <Mekanism-26.3-checkout>` (a sparse clone of `src/additions`, `src/datagen/generated/mekanismadditions`, `src/api/java/mekanism/api/text` and `LICENSE` is enough; Pillow is needed). Models, blockstates, item definitions, loot tables, recipes, tags and names come from the checkout. License: MIT, see `content/mekanismadditions/LICENSE-MEKANISM`.

## Scope (271 blocks)

18 colors (vanilla colors plus `aqua` and `dark_red`), 15 variants each: Plastic, Slick, Glow, Reinforced, Road, Transparent, Stairs, Slabs, Fences, Fence Gates, Glow Stairs/Slabs, Transparent Stairs/Slabs and Glow Panel; plus Obsidian TNT.

| Variant | Template | Values |
|---|---|---|
| Plastic, Road | `stone` | hardness 5, resistance 6 |
| Slick | `stone` | `friction` 0.98 |
| Glow, Glow Stairs/Slabs | `stone`, `stone_stairs`, `stone_slab` | light 10 |
| Reinforced | `stone` | hardness 50, resistance 1200 |
| Transparent (+ Stairs/Slabs) | `glass`, `stone_stairs`, `stone_slab` | `occludes: false` |
| Fence / Gate | `nether_brick_fence` / `oak_fence_gate` | hardness 5, resistance 6 |
| Glow Panel | `small_amethyst_bud` | light 15, own `shapes` per `facing` from `BlockShapes.GLOW_PANEL` |
| Obsidian TNT | `tnt` | hardness 0 |

All: stone sound, map color as `EnumColor`, no tool required.

Colors: Upstream tints the gray textures at runtime (block/item color providers). Horizon content has no color provider. So the generator pre-colors each texture per color (`textures/block/<color>/`) and creates separate models per color (`models/block/<color>/`); the shared `colored/*` models are copied without `tintindex`. On the Glow Panel, only the center plate is colored, as in the original.

## Recipes

Taken over: Slabs, Stairs, Fences, Gates, Roads, Reinforced, Glow, Transparent (shaped/shapeless) and the Slick recipes of the Enrichment Chamber, as far as all Mekanism ingredients are registered. Not included:

- Plastic base recipe and Glow Panels: need `mekanism:hdpe_sheet` (HDPE, M06 chemistry). The generator adds them automatically once the item is in `scripts/registry.json`.
- Recoloring (`neoforge:difference` ingredient) and Painting Machine recipes (`mekanism:painting`, M06).
- Pigment extraction, Balloons, Walkie-Talkie.

## Differences from the original (Horizon limits, nothing faked)

| Original | Here | Reason |
|---|---|---|
| Obsidian TNT: own entity, radius 12, configurable fuse time | explodes like vanilla TNT (radius 4, fuse time 80 ticks) | A custom entity/explosion needs code. A content block can only take over the `tnt` template. A native hook for explosion strength is missing |
| Plastic Road: speed boost (`stepOn`) | normal block | `stepOn` is code; there is no content property for speed |
| Glow/Transparent: `emissiveRendering`, CTM textures, hides covered inner faces of the same color (`skipRendering`) | light through `light`; no CTM, no face skipping | Render code/mod not available |
| Transparent: `isValidSpawn`/`isSuffocating`/`isViewBlocking` off, beacon color | glass template (spawn and beacon like glass) | Cannot be loaded as a property |
| Fence/Slab/Obsidian TNT: extended fluid logging (lava, water) | water as in vanilla | `IStateExtendedFluidLoggable` is code |
| Glow Panel: own block class, placement rules | template `small_amethyst_bud` (6 directions, waterloggable, drops without support) | Behavior similar to the original, minimum size of the support face not identical |
| Configuration (TNT radius/fuse time, baby mobs, spawn rates) | missing | Horizon content has no config |
| Advancements, recipe book entries | missing | Data type not loaded |

## Not built: entities, Balloons, Walkie-Talkies, Spawn Eggs

Investigated only (schema from `cargo horizon-mod`, no Horizon source code available):

- Horizon content can declare `entity_type/*.json` (template + attributes, "living type"), that is, a vanilla creature with vanilla AI and vanilla model under a new ID. Custom behavior, custom models, biome spawns and network data are not part of the schema.
- **Baby mobs** (Creeper, Enderman, Skeleton, Stray, Wither Skeleton, Bogged, Parched): In the original these are separate entity classes with a small model and Mekanism's drops/spawn biomes (`biome_modifier`). As a template, only possible with the vanilla baby variant; spawns need NeoForge biome modifiers. Not built because parity is not reachable.
- **Balloons**: an entity that hangs on a leash from a block or player, rises and pops (`EntityBalloon`, interaction, network sync, color, advancement `pop_pop`). Needs native code (entity tick, right-click on entity, sync); Horizon content has none of that.
- **Walkie-Talkie**: channels, voice radio between players via packet/chat (`ItemWalkieTalkie`, channel components, key). Needs items with state, key polling and networking; according to their capabilities, native mods may "see the keys you hold…/message", but the effort belongs in a separate native task (M13 interop).
- **Spawn Eggs**: depend on the baby entities.

These items stay open in `TODO.md`.
