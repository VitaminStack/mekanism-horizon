# Wave 1 – bundled in-game checklist

Branch `test/wave1-acceptance` (8 session branches merged). Build first:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

The native code of this wave has **never been compiled**. Report compile errors from the first build to the lead session first.
Use a creative world, with energy from Creative Energy Cubes. Details for each item are in the matching `docs/sessions/<name>.md`.

## 1. Loading and content
- [ ] The game starts without loading errors, native mod active.
- [ ] New items/blocks with model and names in the creative inventory: Upgrades, Configurator, 32 Factories, 4 Tier Installers, 4 Universal Cables, Transporters (4 tiers, Restrictive, Diversion), Jetpack, Free Runners, Scuba, Electric Bow, Flamethrower, Dynamic Tank/Valve, Structural Glass, Induction Casing/Port/Cells/Providers, Teleporter, Teleporter Frame, Portable Teleporter, Quantum Entangloporter, Oredictionificator, Formulaic Assemblicator, Crafting Formula, Digital Miner.
- [ ] Additions: recolor at the crafting table, Painting Machine and Pigment Extractor with Plastic; names in de/en (Additions, Tools).
- [ ] Generators: craft, place and break Turbine Valve/Vent, Fission/Fusion Port, Logic Adapter, Fusion Controller (no function).

## 2. Machine framework (`m07-machine-framework.md`)
- [ ] Enrichment Chamber: energy slot, upgrade slots, buttons `RS`, `S/E/M` visible; an old machine from an earlier save loads without loss.
- [ ] An Energy Tablet in the energy slot charges the machine.
- [ ] Speed x8: 20 ticks per operation; Energy upgrade changes capacity; Muffling makes it silent; click to take an upgrade back.
- [ ] Redstone High/Low/Pulse; mode persists after restart.
- [ ] Configurator turns the machine/cube to the clicked side.

## 3. Factories (`m07-factories.md`)
- [ ] Basic Enriching Factory: 3 processes in parallel; full output pauses.
- [ ] Combining/Compressing/Sawing: extra slot, tank, secondary output; Ultimate layout (9 columns).
- [ ] Tier Installer: Chamber → Basic Factory keeps its contents; nothing duplicated or lost.
- [ ] Restart/chunk unload keeps progress, energy, tank; breaking creates no duplicate.

## 4. Cables and transporters (`m08-cables-transport.md`)
- [ ] Solar Generator → cable → cube charges; arms connect.
- [ ] Transporter chest A → B (sneak+right-click "Pull"), tier speed, full target, Diversion with redstone.
- [ ] Breaking with items: items drop; restart: items keep moving.

## 5. Gear (`m11-gear.md`)
- [ ] Fill the Jetpack at a Chemical Tank (hydrogen), fly, modes Hover/Vector/Disabled.
- [ ] Free Runners: no fall damage (energy drops), step assist.
- [ ] Scuba mask + tank underwater; Electric Bow (120 FE/shot); Flamethrower (cone, fire).
- [ ] Enchantments are kept after loading/use.

## 6. Multiblocks (`m09-multiblocks.md`)
- [ ] Dynamic Tank 3×3×3 forms, Valve in a face, pipe fills it, breaking dissolves it, contents return.
- [ ] Induction Matrix 3×3×3 with cell + provider forms; Port input/output; capacity display.
- [ ] Chunk unload does not dissolve structures.

## 7. Teleporter (`m08-teleport-frequency.md`)
- [ ] Two Teleporters with frames, same channel: the player is teleported, energy drops; too little energy → no teleport.
- [ ] Portable Teleporter: switch channel (sneak), teleport.
- [ ] Two Entangloporters with the same frequency share energy/fluid/chemical/item; restart keeps the frequencies.

## 8. Special machines (`m08-special-machines.md`)
- [ ] Oredictionificator: filter on a tag, input becomes the chosen item; filter persists after restart.
- [ ] Digital Miner: 17 bounding blocks, radius/minY/maxY, mining with filter, Silk Touch, ejection at the back, restart.
- [ ] Formulaic Assemblicator: only block + item, no function (blocker: no vanilla crafting recipes for native mods).

## Known gaps in this wave
- Logistical Sorter, Thermal Evaporation Plant, Seismic Vibrator/Reader: not built.
- Factories and framework each have their own upgrade logic (`factory.rs`, `upgrade.rs`): merge them after acceptance.
