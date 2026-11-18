# Changelog

All notable changes, grouped by milestone from [TODO.md](TODO.md). Source: Git history and merged pull requests (#1 to #16). Known gaps: [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md). The format loosely follows [Keep a Changelog](https://keepachangelog.com/). There is no tagged release yet; the content mods `mekanism` and `mekanismgenerators` are at 0.2.0, `mekanismtools` and `mekanismadditions` at 0.1.0.

Base: Mekanism 26.3 (Mekanism 10.8.0 alpha), Horizon Nightly 2026-11-08 `fdc87ae`.

## [Unreleased] (as of 2026-11-11)

### M06 Nutrition and pigments
- Nutritional Liquifier, Pigment Extractor, Pigment Mixer and Painting Machine (native, table `machine::MACHINES`) with 40 nutrition, 275 extraction, 16 mixing and 288 painting recipes, item `dye_base`; not tested in game (test plan in `docs/TESTLOG.md`).

### M06 Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator
- Three native chemistry machines with recipes, models, sounds and names from upstream (`mekanism:reaction`, `centrifuging`, `activating`), new items `substrate` and `hdpe_pellet`, Generators chemicals (Deuterium, Tritium, Fusion Fuel) in the chemistry table. Not tested in game, see [docs/CHEMISTRY.md](docs/CHEMISTRY.md) and [docs/TESTLOG.md](docs/TESTLOG.md).
- Fix: missing bracket in `kind()` (`lib.rs`) from a merge of `main`; the native crate did not compile.

### M06 In-game acceptance (2026-11-13)
- Nutrition/pigments, pipes/tubes/conductors and Advanced Chemistry accepted in game (results in `docs/TESTLOG.md`, "M06 acceptance").
- Fix: pipes fill tanks and machines at the end at the full rate (before, about half because of the fill-fraction rule); tubes no longer get an arm to Fluid Tanks.

### M11 Gear and utilities
- Hook audit and feasibility matrix of all M11 items in `docs/ITEM_HOOKS.md` (Horizon `fdc87ae`, API 411).
- Atomic Disassembler (energy buffer, weapon bonus, vein mode), Canteen, Gauge Dropper and Dictionary as native items (`items.rs`, `gear.rs`) with assets, recipes and names from upstream. Items that are not feasible (Radiation, Modules, QIO) stay unregistered, see `docs/KNOWN_ISSUES.md`.

### M08 Storage
- Bins (Basic to Creative), Personal Chest and Personal Barrel (owner protection without Security Desk). Chargepad is open, see [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md).

### M13 Documentation
- Install guide, known issues, contributing guide, license review and this changelog.

### M04 Generators
- Gas-Burning Generator (native): chemical tank, fuel values for Hydrogen/Ethene, GUI, sound; `electrolytic_core` and `electrolytic_separator` are craftable.

### M06 Chemistry and machines
- Electric Pump and Fluidic Plenisher (native, `pump.rs`/`pumps.rs`): water and lava out of/into the world, tank 10,000 mB, breadth-first search as upstream.
- Chemical Dissolution Chamber, Chemical Washer and Chemical Crystallizer; crystal, shard and dirty dust items for 7 metals; ore chain (1 ore to 5 ingots) tested in game, see [docs/ORE_PROCESSING.md](docs/ORE_PROCESSING.md).
- Chemical Oxidizer and Chemical Infuser ([#16](https://github.com/VitaminStack/mekanism-horizon/pull/16)). Chemical output goes to output tanks, two chemical inputs; a deadlock when ejecting under the recipe lock was fixed.
- Purification Chamber and Chemical Injection Chamber with clump items ([#14](https://github.com/VitaminStack/mekanism-horizon/pull/14)).
- Fluid and chemical tables from upstream, Fluid and Chemical Tanks (auto-eject, dumping modes, buckets, fluid rendering, GUI) ([#10](https://github.com/VitaminStack/mekanism-horizon/pull/10), [#11](https://github.com/VitaminStack/mekanism-horizon/pull/11)).

### M12 Tools and Additions
- Content mod `mekanismadditions` 0.1.0: 271 blocks (Plastic, Slick, Glow, Reinforced, Road, Transparent, Stairs, Slabs, Fences, Gates, Glow Panels, Obsidian TNT), textures pre-colored per color ([#15](https://github.com/VitaminStack/mekanism-horizon/pull/15)).
- Content mod `mekanismtools` 0.1.0: 91 items (Paxel, tool and armor sets in 6 materials, shields, spears, horse and nautilus armor), 107 recipes ([#13](https://github.com/VitaminStack/mekanism-horizon/pull/13)).

### M05 Machines
- Osmium Compressor, Combiner and Precision Sawmill ([#12](https://github.com/VitaminStack/mekanism-horizon/pull/12)). Fixes: sawdust item, missing `c:` tags.

### P0 Framework
- Shared machine framework: one tick, one recipe cache, energy acceptors; multiblock fixes (bounding blocks) ([#9](https://github.com/VitaminStack/mekanism-horizon/pull/9)).

### M02 Machines and resources
- Enrichment Chamber and Crusher with machine sounds ([#8](https://github.com/VitaminStack/mekanism-horizon/pull/8)).
- Metallurgic Infuser, infuse types and infusing recipes, block sounds as in Mekanism ([#5](https://github.com/VitaminStack/mekanism-horizon/pull/5)).
- Raw materials: ores, ingots, dust, blocks, alloys and worldgen in 26.3 format, assets, recipes, tags and names from upstream (en_us, de_de) (commit `cd48129`, preparatory work in [#1](https://github.com/VitaminStack/mekanism-horizon/pull/1) to [#3](https://github.com/VitaminStack/mekanism-horizon/pull/3)).

### M04 Generators
- Heat Generator and Solar Generator ([#7](https://github.com/VitaminStack/mekanism-horizon/pull/7)).
- Bio Generator with Bio Fuel, Advanced Solar Generator and Wind Generator (multiblock with rotor) ([#8](https://github.com/VitaminStack/mekanism-horizon/pull/8)).

### M03 and M01 Energy
- All five Energy Cube tiers and Energy Tablet ([#2](https://github.com/VitaminStack/mekanism-horizon/pull/2)).
- Basic Energy Cube complete: GUI, side I/O, item energy, saving/chunk reload ([#4](https://github.com/VitaminStack/mekanism-horizon/pull/4), [#5](https://github.com/VitaminStack/mekanism-horizon/pull/5)). Debug energy source and sink for testing.

### M00 Project base
- Basic Energy Cube prototype v0.2.0 (content + native block entity and energy) and project documentation ([#1](https://github.com/VitaminStack/mekanism-horizon/pull/1)).
- CI: content validation and energy tests (`.github/workflows/ci.yml`); workflow that generates the progress block in `README.md` from `TODO.md` ([#3](https://github.com/VitaminStack/mekanism-horizon/pull/3), [#6](https://github.com/VitaminStack/mekanism-horizon/pull/6)).
- Build scripts for content and native, test log `docs/TESTLOG.md`.

## Pull Requests

| PR | Date | Title |
|---|---|---|
| #1 | 2026-11-08 | Basic Energy Cube prototype v0.2.0 and project documentation |
| #2 | 2026-11-09 | Native test, test log and debug energy blocks (N05, N08, N10) |
| #3 | 2026-11-09 | N wrap-up: Energy Cube in-game test (N05-N10) |
| #4 | 2026-11-09 | M01: Basic Energy Cube |
| #5 | 2026-11-09 | Metallurgic Infuser, infusing recipes and block sounds (M02) |
| #6 | 2026-11-09 | docs: sync confirmed progress |
| #7 | 2026-11-09 | Heat and Solar Generator (M04) |
| #8 | 2026-11-09 | M02/M04: Enrichment Chamber, Crusher, Bio, Advanced Solar and Wind Generator |
| #9 | 2026-11-09 | P0: machine framework, recipe cache, energy acceptors; multiblock fixes |
| #10 | 2026-11-09 | M06: fluid/chemical tables, fluid and chemical tanks |
| #11 | 2026-11-09 | docs: tick confirmed TODO items, update README progress |
| #12 | 2026-11-10 | M05: Osmium Compressor, Combiner, Precision Sawmill |
| #13 | 2026-11-10 | feat(m12): Mekanism Tools content mod |
| #14 | 2026-11-10 | feat(m06): Purification Chamber and Chemical Injection Chamber |
| #15 | 2026-11-10 | feat: Mekanism Additions content mod (M12) |
| #16 | 2026-11-10 | feat(m06): Chemical Oxidizer and Chemical Infuser |

## 0.2.0 (2026-11-08)

First prototype: Osmium (ore, ingot, block), Basic Energy Cube with native companion, Windows build scripts, documentation.
