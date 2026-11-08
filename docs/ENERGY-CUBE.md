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

1. Start Horizon using the newly built native game and enable the `mekanism`
   **content** mod under Options > Mods.
2. Restart Horizon after changing block/item registration.
3. In a test world, run `/give @s mekanism:basic_energy_cube 1`.
4. Place it; break it and check that it drops an item.
5. With another mod or a future test fixture that can use Horizon's
   energy capability, insert energy through a face, save and reload, and
   check the amount persists; extract up to 4,000 in one call.

## Current limitations

- **Not yet verified**: successful native game compilation, game-side
  energy insertion, persistence, or interoperability. The test model is
  separate from the runtime native hooks.
- No GUI, visible charge bar, wrench/side configuration, redstone modes,
  auto-eject, charging items, generator, upgrades or other tiers yet.
- No passive transfer between cubes: another native machine or a future
  power network must actively call Horizon's `native_move_energy`/insert
  helpers. The cube only **exposes** the capability.
- On breaking, the prototype does **not** preserve the stored charge
  inside the dropped item. Save a test world before testing changes.
- The development recipe is **NOT** Mekanism's authentic recipe because
  Energy Tablets / Steel Casing have not yet been ported.
- The cube texture is a vanilla placeholder, not Mekanism artwork.

## Licensing

This is an unofficial prototype. The code is independently written for
Horizon; any future reuse of Mekanism assets must respect the original
project's MIT copyright/notice requirements.
