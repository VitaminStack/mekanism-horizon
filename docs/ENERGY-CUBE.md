# Mekanism Horizon - Basic Energy Cube (experimental)

## What is included

- Content mod `content/mekanism`: placeable `mekanism:basic_energy_cube`,
  German/English name, block/item model (temporary copper placeholder), loot,
  pickaxe tags, and a **temporary development recipe**.
- Native companion `native/mekanism_native`: a trait-based Horizon hook module.
  It creates a saved block entity (`block_entity`, hook API 15) and declares
  Horizon's energy capability (`block_entity_energy`, hook API 142).
- The Horizon engine saves an `energy` **Int** in block entity NBT and owns
  insertion/extraction through the official helpers. No custom world serializer.
- Standalone Rust reference tests in `logic/energy` for capacity, transfer
  limits, clamping and save/reload.

## Energy defaults

| Attribute | Value |
| --- | --- |
| Basic cube capacity | 4,000,000 (Mekanism EnergyCubeTier.BASIC default) |
| Max insert per call | 4,000 |
| Max extract per call | 4,000 |
| Input/output faces | All six (development default; configurable faces later) |

Source: https://github.com/mekanism/Mekanism/blob/master/src/main/java/mekanism/common/tier/EnergyCubeTier.java

These limits are per API call, **not a guaranteed per-tick limit**. The
native Horizon API uses i32 amounts, so this tier fits without loss. Energy
units are only interoperable with other mods using Horizon's energy API;
full Mekanism Joule/FE conversions have not been implemented.

## Building on Windows / Visual Studio 2026

The normal Ctrl+B content build (`scripts/Build-Content.ps1`) still builds
the `.hmod` and, if you configured it, copies it to your game instance.
Build and enable the content mod first. Then use the **native companion**:

```powershell
# From your repository root:
cargo test --manifest-path .\logic\energy\Cargo.toml
.\scripts\Test-Content.ps1
.\scripts\Build-Content.ps1

# Separate native game build; review the source before trusting it:
.\scripts\Build-Native.ps1 -TrustNativeCode
```

If PowerShell blocks local scripts, run for this session only:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
```

Native code is **not** contained in `mekanism.hmod`! The launcher builds
a new Horizon executable using `native/mekanism_native/` and the currently
selected instance. Follow its native-code trust prompts; do not publish
the project as fully working until the native game build and in-game tests pass.

The launcher command used is documented in Horizon's `docs/native-mods.md`:
`horizon-launcher native-build <folder> --trust-native-code`.
The selected Horizon build must support native hook API >= 142. You may
need to update Horizon to a newer Nightly and to compile its game source.

### In-game smoke test

1. Rebuild **both** the content `.hmod` and native companion (the content
   menu type and new block template are required) and enable the content mod.
   Start Horizon using the newly built native game.
2. Restart Horizon after changing block/item registration.
3. In a test world, run `/give @s mekanism:basic_energy_cube 1`.
4. Place it; break it and check that it drops an item.
5. With another mod or a future test fixture that can use Horizon's
   energy capability, insert energy through a face, save and reload, and
   check the amount persists; extract up to 4,000 in one call.

## Behaviour (M01, all five tiers)

- **Model**: Mekanism's own frame, corner and port textures. Its model loader is
  replaced by plain models (`scripts/import_energy_cube.py`): a multipart
  blockstate shows a port on each side whose config is not "none"
  (`port_<side>` state properties). Status LEDs and the inner energy core
  (Mekanism's block entity renderer) are not drawn.
- **Placement**: the front faces the placer (`facing`, all six directions).
- **Side config** by relative side (front, back, left, right, top, bottom),
  saved as `mekanism_sides`; default as Mekanism: output at the front, input elsewhere.
- **GUI**: right click opens Horizon's content-defined `mekanism:energy_cube`
  container and custom native screen. Unlike the older `Host::open_menu`
  helper, this path also works in a local singleplayer world. The screen shows
  exact stored energy, percentage as a bar, transfer rate, ejection and the
  six sides. Left click on a side: next mode; right click: previous.
- **GUI backend**: content block template `minecraft:barrel` (which has a
  built-in `facing` state and a native use/open action); the actual block model
  remains Mekanism's. All five cube tiers use this template. A single hidden
  container slot (`results: [0]`) keeps player items out of the machine GUI;
  side changes go through `native_screen_click` / `native_menu_button` and
  sync as block-entity `data`. Values above 32767 are split across two 16-bit
  data slots. No `right_click_block_v2` cancel and no LAN-only chest menu.
- **Ejection**: every tick the cube pushes up to its transfer rate out of its
  output sides into neighbouring Mekanism cubes whose facing side is input.
  What one cube loses the other gains in the same step.
- **Breaking**: the item keeps energy and side config (`minecraft:custom_data`:
  `horizon_energy`, `mekanism_sides`), also when blown up, as Mekanism's loot
  table copies them; placing it restores both. Cube items hold energy through
  Horizon's item energy API; the tooltip shows it.

## Current limitations

- Horizon's energy faces (`insert_faces`/`extract_faces`) are per block type,
  not per block entity: other mods may fill or drain **any** side, whatever the
  side config says.
- A trait mod has no energy helper for other mods' blocks (`native_move_energy`
  is game-internal), so ejection reaches only Mekanism cubes.
- No charge/discharge item slots, redstone control or security yet.
- Crafting does not carry the inputs' energy over (`mekanism:mek_data` is plain shaped crafting).
- The installed `cargo-horizon-mod` rejects `occludes`. The cubes now use the
  menu-opening barrel template instead of glass; test the renderer around the
  cube's open frame after rebuilding the content mod.

## Licensing

This is an unofficial prototype. The code is independently written for
Horizon; any future reuse of Mekanism assets must respect the original
project's MIT copyright/notice requirements.
