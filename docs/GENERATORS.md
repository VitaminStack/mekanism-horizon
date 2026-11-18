# Mekanism: Generators, comparison with upstream (M12)

Comparison of `content/mekanismgenerators` with Mekanism Generators 26.3 (upstream commit `31bb4906`). All assets, recipes and tags come from `scripts/import_upstream.py` and `scripts/import_generator_models.py`; registration is in `scripts/gen_resources.py` (Generators section).

Order after changes: `gen_resources.py`, `import_upstream.py <Mekanism>`, `import_generator_models.py <Mekanism>`, `scripts/Build-Content.ps1`.

## Taken over (content, no function except the block itself)

| Upstream ID | Status |
|---|---|
| `heat_generator`, `solar_generator`, `bio_generator`, `advanced_solar_generator`, `wind_generator` | work (native, M04), recipes, models, loot, lang |
| `gas_burning_generator` | works (native, M04): 1,000 mB chemical tank (Horizon block entity tank), fuel values from `chemical_attribute_fuel` (Hydrogen 2, Ethene 8 FE per mB, max. 10 mB/t), storage 2,000 FE, energy from the front, GUI with gauge, energy bar and burn rate, sound `tile.generator.gas`. Recipe needs `mekanism:electrolytic_core` (now craftable) |
| `solar_panel` | item, recipe |
| `turbine_blade`, `hohlraum` | item only, recipe (Hohlraum: Metallurgic Infuser, recipe in the namespace `mekanismgenerators`); Hohlraum stores no chemicals |
| `turbine_casing`, `turbine_rotor` (model `fence_post`), `fission_reactor_casing`, `fusion_reactor_frame`, `reactor_glass`, `laser_focus_matrix`, `electromagnetic_coil`, `rotational_complex`, `saturating_condenser`, `fission_fuel_assembly`, `control_rod_assembly` | block only: model, texture, loot, lang, recipe. No block entity and no multiblock logic. Turbine Blade cannot be put into the rotor |

Block strengths as upstream: casings/frame/glass 5 / 9, the rest 3.5 / 16, Laser Focus Matrix 3.5 / 4.8.

## Blocked or postponed (needs native code)

| Upstream ID | Reason |
|---|---|
| Industrial Turbine (multiblock) with `turbine_valve`, `turbine_vent` | Forming, steam, rotor/blade count, condensers, energy output. Valve and Vent are not registered yet because they have no function without the multiblock |
| Fission Reactor with `fission_reactor_port`, `fission_reactor_logic_adapter` | Multiblock, coolant/fuel/waste, heat, GUI |
| Fusion Reactor with `fusion_reactor_controller`, `fusion_reactor_port`, `fusion_reactor_logic_adapter` | Multiblock, plasma, laser, Hohlraum ignition. The frame is craftable only if `#c:pellets/polonium` exists (missing, M07/radioactivity) |
| Chemicals `deuterium`, `tritium`, `fusion_fuel`, recipe `chemical_infusing/fusion_fuel` | Chemicals only in the native chemistry table (`gen_chemicals.py` knows only the namespace `mekanism`); Tritium/Deuterium recipes need Rotary Condensentrator, Electrolytic Separator, Solar Neutron Activator |
| Buckets and fluids `bioethanol`, `deuterium`, `tritium`, `fusion_fuel` | Horizon knows only vanilla fluids (see `docs/CHEMISTRY.md`) |
| Modules `geothermal_generator_unit`, `solar_recharging_unit` | Mekanism module system is missing |
| Superheating Element (blockstate upstream, no registration entry) | not registered |
| Damage types (`damage_type`), advancements, EMI aliases | not taken over (native, or no counterpart); the importer copies the structure tags for the multiblock check |

Namespace and dependency: The mod `mekanismgenerators` uses textures (`mekanism:block/models/ports`), tags and recipe ingredients from `mekanism`; `mekanism` must be loaded too. The manifest has no dependency field, only a note in the description. Enable both mods in `horizon-mods.txt`.

## Gas-Burning Generator (native)

Logic in `generator.rs` (`gas_tick`, table `GAS_FUELS`), host side in `generators.rs`. Fuel is what upstream lists through the data map `chemical_attribute_fuel`: Hydrogen (energy density 2) and Ethene (8), at most 10 mB per tick each. Each tick, `min(ceil(10 x fill level), supply)` mB are burned, limited by the free energy storage (2,000 FE); `ceil(energy / density)` mB are used.

Not ported, Horizon limits:
- The tank is reachable from all sides (Mekanism: all except the front) because Horizon's faces apply per block type, not per facing. It accepts only fuels (`block_entity_accepts_fluid`).
- No chemical item slot (filling/emptying Chemical Tank items), no energy slot, no redstone control.
- Fuels from other mods (data map) are missing; the table is fixed.
