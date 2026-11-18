# Licenses and Attribution

Check of 10.11.2026 (TODO M13: "check license, copyright and asset attribution before releases"). This is not legal advice. Result: the base is clean (everything MIT), but there are gaps that should be closed before a release.

## What exists

| File / area | License | Finding |
|---|---|---|
| `LICENSE` (repository root) | MIT, `Copyright (c) 2026 SozialHilfe` | present |
| `content/mekanism/LICENSE-MEKANISM` | MIT, `Copyright (c) 2017-2025 Aidan C. Brady` | present |
| `content/mekanismgenerators/LICENSE-MEKANISM` | identical (checked with `cmp`) | present |
| `content/mekanismtools/LICENSE-MEKANISM` | identical | present |
| `content/mekanismadditions/LICENSE-MEKANISM` | identical | present |
| `logic/energy/Cargo.toml`, `native/mekanism_native/api/Cargo.toml` | `license = "MIT"` | present |

Mekanism is under the MIT license, so passing on code, textures, models and sounds from it is allowed, as long as the copyright notice and license text go with them. The four `LICENSE-MEKANISM` files are in the source folder of each content mod.

## Origin of the content per content mod

| Mod | Taken from Mekanism (script) | Own |
|---|---|---|
| `mekanism` | Textures (188 PNG), models, blockstates, recipes, tags, names, 10 sounds (`import_upstream.py`, `import_energy_cube.py`) | Registry declarations, loot, worldgen (`gen_resources.py`), debug blocks |
| `mekanismgenerators` | Models, textures (8 PNG), 4 sounds (`import_upstream.py`, `import_generator_models.py`) | Registry, loot |
| `mekanismtools` | Textures (121 PNG), equipment assets, recipes, tags, names (`gen_tools.py`) | Material tables from Mekanism's default values, components |
| `mekanismadditions` | Models, blockstates, loot, recipes, tags, names; textures **pre-colored** from upstream grayscale (`gen_additions.py`, Pillow) | Block declarations |
| `native/mekanism_native`, `logic/energy` | Values (tier capacities, recipe times, energy costs) after the Mekanism source code; the Rust code itself is written from scratch | all code |

## Gaps and recommendations

1. **No Mekanism notice in the root `LICENSE`, `README` and in the mod archives.** `LICENSE-MEKANISM` is only in the source folder. Whether `cargo horizon-mod build` packs the file into the `.hmod` is not checked. Check it and, if needed, add a notice in `description` or in the release notes. A `NOTICE`/"credits" section in `README.md` is missing (README names Mekanism only as a template).
2. **Copyright period.** The copied license says `2017-2025`. The upstream branch 26.3 may carry a newer year. At the next upstream comparison, copy `LICENSE` again (`import_upstream.py` reads it from the checkout) and compare.
3. **Owner names are inconsistent.** `LICENSE` names `SozialHilfe`, `horizon-mod.toml` and the GitHub repository name `VitaminStack`. Clarify the owner and enter it consistently.
4. **Third-party contributions in Mekanism.** Some Mekanism assets come from contributors (textures, sounds, models). The Mekanism repository declares MIT for the whole project. The per-file origin (e.g. sounds, Blockbench models of the generators) is not documented individually here. Before a release, check the upstream notes on assets (README/credits/`docs`) and, for deviating licenses, remove or replace the file.
5. **Derived textures.** The Additions textures are colored derivatives. This is allowed under MIT, but the mod notice should say "derived from Mekanism textures".
6. **Missing attribution in the game.** Neither the mod descriptions (`horizon-mod.toml`) nor the mod list name the Mekanism authors. The description says "Unofficial". Recommendation: add the sentence "Based on Mekanism (MIT, Aidan C. Brady and contributors)".
7. **Horizon dependency.** `native/.../api/Cargo.toml` pulls in `horizon-client` from `horizon-rust/horizon`. Its license is not checked here. For release artifacts that contain Horizon source code or are built from it, check the Horizon license.
8. **Horizon drafts.** `docs/upstream/*.md` are own text drafts for Horizon, not copies. Not critical.
9. **Trademarks.** "Mekanism" is the project name of the original. The port is unofficial (README says so). Keep that and repeat it on release pages.
10. **Rust dependencies.** `logic/energy` has a `Cargo.lock`. Dependency licenses are not evaluated (check e.g. with `cargo deny` / `cargo license`).

## Checklist before a release

- [ ] Clarify the owner in `LICENSE` (point 3).
- [ ] Compare `LICENSE-MEKANISM` against the current upstream (point 2).
- [ ] Check whether the license file ends up in the `.hmod`, otherwise add it (point 1).
- [ ] Add attribution in README and mod descriptions (points 1, 6).
- [ ] Spot-check the asset origin against upstream (point 4).
- [ ] Check the Horizon license and Rust dependencies (points 7, 10).
