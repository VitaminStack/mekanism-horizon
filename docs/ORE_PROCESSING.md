# Ore processing (M06)

The chain of Mekanism's 5x processing as it runs here. The amount and time of the machines are in
`native/mekanism_native/api/src/machine.rs` (values from Mekanism 26.3), the recipes in
`content/mekanism/data/mekanism/recipe/processing/<metal>/`.

## Chain (iron, 1 ore)

| Step | Machine | Recipe type | In | Out |
|---|---|---|---|---|
| 1 | Chemical Dissolution Chamber (16 FE/t, 100 ticks) | `dissolution` | 1 ore + 1 mB sulfuric acid per tick (100 mB) | 1,000 mB dirty slurry (`dirty_iron`) |
| 2 | Chemical Washer (5 FE/t, 1 pass per tick) | `washing` | 1 mB dirty slurry + 5 mB water | 1 mB clean slurry (`clean_iron`) |
| 3 | Chemical Crystallizer (15 FE/t, 200 ticks) | `crystallizing` | 200 mB clean slurry | 1 crystal (`crystal_iron`) |
| 4 | Chemical Injection Chamber | `injecting` | 1 crystal + 1 mB hydrogen chloride per tick | 1 shard (`shard_iron`) |
| 5 | Purification Chamber | `purifying` | 1 shard + 1 mB oxygen per tick | 1 clump (`clump_iron`) |
| 6 | Crusher | `crushing` | 1 clump | 1 dirty dust (`dirty_dust_iron`) |
| 7 | Enrichment Chamber | `enriching` | 1 dirty dust | 1 dust (`dust_iron`) |
| 8 | Furnace / Blast Furnace (Energized Smelter is missing, see below) | `smelting` | 1 dust | 1 ingot |

1,000 mB of slurry make 5 crystals and, in the end, **5 ingots per ore** (vanilla: 1; enrichment route: 2).
Raw ore blocks give 6,000 mB of slurry at 2 mB acid per tick. Tested in game on 2026-11-11
([TESTLOG.md](TESTLOG.md), section "Dissolution Chamber, Washer, Crystallizer and ore chain").
Gold, copper, tin, lead, osmium and uranium have the same recipes (7 metals each: crystal, shard, clump,
dirty dust); uranium also has hydrofluoric acid recipes, which only run with the Pressurized Reaction Chamber.

## Sulfuric acid

As in the original, the acid comes from the Chemical Infuser (sulfur trioxide + water vapor), sulfur trioxide from
sulfur dioxide + oxygen, sulfur dioxide from sulfur dust in the Chemical Oxidizer, water vapor from water in the
Rotary Condensentrator (mode fluid to chemical), oxygen from the Electrolytic Separator. Every stage
is tested in game on its own; the full chain from water to acid is **not**: the test filled
Dissolution and Washer with `/data merge` (acid, water).

## Chemicals and fluids in these machines

- Dissolution: tank 0 (acid, 10,000 mB, can be filled by neighbors and pipes), tank 1 (slurry, 10,000 mB, is ejected into all neighbors).
  The acid is a tank, not a buffer as in Purification/Injection (`Machine::chemical_tank`).
- Washer: tank 0 water, tank 1 dirty slurry, tank 2 clean slurry (10,000 mB each), `machine::wash`.
- Crystallizer: one input tank (10,000 mB), item in the output slot.

## Limits (Horizon / not built yet)

- **Energized Smelter** is blocked (Horizon gives native mods no `minecraft:smelting` recipes, see TODO M05); step 8 goes through a vanilla furnace.
- No pipes/tubes: the chain runs through directly adjacent machines (ejection into neighbors) or by hand.
- No water bucket for machines (Horizon's bucket hook only applies to tanks): water comes from a Fluid Tank next to the Washer.
- Chemical item slots (fill/empty tanks by item), energy slot, upgrades, side config and redstone are missing, as on the other chemistry machines.
- A tank accepts every chemical that occurs in a recipe of the machine (Horizon's `accepts` hook knows neither the position nor the other tank): a wrong slurry in the Washer input blocks it until it is emptied.
- Recipe change (ore type) at half progress: Dissolution and Crystallizer keep or set the progress following the original pattern, not tested separately.
- Crafting of the three machines (Dissolution/Washer: Chemical/Fluid Tank, Crystallizer: fluorite, Ultimate Circuit) exists through the import, not checked in game; `mek_data` becomes normal shaped crafting.
