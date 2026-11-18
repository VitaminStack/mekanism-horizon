# Mekanism: Tools (M12)

Content mod `content/mekanismtools` (namespace `mekanismtools`, needs `mekanism` for the ingots). Pure content, no native code: Horizon items are a vanilla template (`template`) plus `components`, and tool values, armor, shield and spear are vanilla data components.

Generated with `python scripts/gen_tools.py <Mekanism-26.3-checkout>` (a sparse clone of `src/tools` and `src/datagen/generated/mekanismtools` is enough). The material tables in the script are Mekanism's default values (`*MaterialDefaults`, `ToolsMaterialConfig`); assets, recipes, tags and names are copied from the checkout. License: MIT, see `content/mekanismtools/LICENSE-MEKANISM`.

## Scope (91 items)

- Paxel: Wood, Stone, Copper, Iron, Gold, Diamond, Netherite (durability = 2x vanilla, damage = axe base + 1).
- For each of Bronze, Lapis Lazuli, Osmium, Refined Glowstone, Refined Obsidian, Steel: Axe, Hoe, Pickaxe, Shovel, Sword, Paxel, Spear, Shield, Helmet, Chestplate, Leggings, Boots, Horse Armor, Nautilus Armor.
- Durability, mining speed, attack values, enchantability, repair material (`#c:ingots/...`), armor, toughness, knockback resistance, fire resistance (Refined Obsidian, Netherite Paxel), spear values and shield values (block delay, damage threshold) are components in `data/mekanismtools/horizon/item/`.
- Mining tier through the block tags `mekanismtools:incorrect_for_<material>_tool` (Bronze/Osmium: iron, Lapis: stone, Steel/Refined Glowstone: diamond, Refined Obsidian: netherite).
- 107 recipes (tools, armor, shields, Paxel, ingot smelting), tags (`c:tools/paxel`, `minecraft:pickaxes`, `piglin_safe_armor` ...), armor equipment assets with textures, item models, `en_us`/`de_de` names.

## Differences from the original (Horizon limits, nothing faked)

| Original | Here | Reason |
|---|---|---|
| Paxel acts as axe **and** shovel (`PaxelItem.useOn`) | axe action only (strip bark); grass is not turned into paths | Content items have no code; `minecraft:block_transformer` takes one transformation. Needs a native hook; `use_item_on` (API 13) exists, see [ITEM_HOOKS.md](ITEM_HOOKS.md), but is not built here yet |
| `mekanismtools:paxel` recipe averages the durability of the ingredients | `minecraft:crafting_shaped`, Paxel starts undamaged | No custom recipe type in Horizon content |
| Shield with its own texture (`mekanismtools:shield` special model) | shield shows the vanilla shield model; values (durability, block delay, threshold) are correct | A custom item model type is NeoForge/Mekanism code |
| Armor trims with Mekanism palettes (`neoforge:trimmed_armor`, `trim_overrides`) | vanilla model, no Mekanism palette | NeoForge only |
| `ToolsDataComponents.DISPLAY_HP`, durability tooltip | missing (vanilla durability bar present) | A custom component needs code |
| Mob spawn equipment (`MobEquipmentHelper`), configuration, advancements, recipe book aliases | missing | Code, or data types that Horizon content does not load |
| Configurable material values | fixed default values | Horizon content has no config; change values by regenerating |

Whether Horizon reads the items for armor/shield values identically on a multiplayer server is not tested.
