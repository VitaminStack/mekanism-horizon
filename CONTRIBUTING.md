# Contributing

Mekanism Horizon is an unofficial port of [Mekanism](https://github.com/mekanism/Mekanism) (branch `26.3`) to [Minecraft Horizon](https://github.com/horizon-rust/horizon). The repository is public and licensed under MIT ([LICENSE](LICENSE); details on third-party content in [docs/LICENSES.md](docs/LICENSES.md)). Contributions are welcome.

## Workflow: fork and pull request

1. Fork the repository, clone your fork, create a branch from `main`. Naming scheme: `feat/m06-...`, `fix/...`, `docs/...`.
2. Make your change, check it locally (see below), commit in small commits. Commit style: Conventional Commits with the milestone, for example `feat(m06): Chemical Oxidizer and Chemical Infuser`.
3. Push the branch, open a pull request against `main`. Description: what, why, how tested (tests, in-game evidence, log or screenshot).
4. `main` is protected: changes only by pull request. **0 reviews** and **no status checks** are required. So check yourself that CI (`.github/workflows/ci.yml`) is green, and do not wait for an approval before you test properly.
5. After the merge, a workflow updates the progress block in `README.md` from `TODO.md`.

Rules from [TODO.md](TODO.md): An item is ticked only when it is **confirmed**, that is, tested in game with real inputs and outputs, not just built. Put open gaps as a note on the item and in [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md). Check original behavior and registry IDs against Mekanism 26.3. Keep content (`.hmod`) and native code separate.

License: Anyone who takes code or assets from Mekanism keeps the copyright and license text ([docs/LICENSES.md](docs/LICENSES.md)). No content from other sources without a suitable license.

## Tests

```powershell
python scripts/validate_content.py
cargo test --manifest-path logic/energy/Cargo.toml
```

Both also run in CI. `scripts/Test-Content.ps1` calls the validation. After every content change, also build (this checks with `cargo horizon-mod check`):

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

The crate `native/mekanism_native/api` contains unit tests, but it can only be built through the Horizon Launcher (it depends on Horizon's client). Build native changes with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

This runs native code with your user rights, see [docs/INSTALL.md](docs/INSTALL.md). This also applies to pull requests from others: read the code first, then build.

## In-game testing with Agent Control

On a native build, the launcher puts the tool `horizon-ctl.exe` in place:

```
%APPDATA%\horizon-launcher\versions\<native-version>\horizon-ctl.exe   state | command | setblock | quit
```

- Start Horizon through the launcher, open a test world (Creative, energy from Creative Energy Cubes).
- Before the first world command, wait for `player.block_pos` in `state`. Earlier commands crash Horizon.
- Use `command`/`setblock` to place machines, `/give`/`item replace` for inputs, `/data get block` for state. Check restart and chunk unload (`tp` far away and back).
- Record results as a table in [docs/TESTLOG.md](docs/TESTLOG.md): date, build name, test, result (`ok`, `failed`, `pending`). Bundle several related tests; do not log every single step.

New content mods start disabled in Horizon: enable them in `horizon-mods.txt` as `mod.<id>:true`.

## Generated content: order of the scripts

Resource content is **generated**. Do not change it by hand. Adjust the generator and run it again. Order:

```powershell
python scripts/gen_resources.py
python -I scripts/import_upstream.py <path-to-Mekanism-26.3-checkout>
python -I scripts/import_generator_models.py <path-to-Mekanism-26.3-checkout>
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

1. `gen_resources.py` writes the Horizon part: registry declarations, loot, ore generation, `scripts/registry.json`.
2. `import_upstream.py` copies assets, recipes, tags, sounds and names from the upstream checkout (a sparse clone is enough; paths are in the script's docstring).
3. `import_generator_models.py` builds the generator models (wind, solar and so on) from the upstream models.

The order matters because step 2 reads the `registry.json` that step 1 wrote, and step 3 builds on the imported assets. More generators: `gen_chemicals.py`, `import_energy_cube.py`, `gen_tools.py` (tools, [docs/TOOLS.md](docs/TOOLS.md)) and `gen_additions.py` (additions, [docs/ADDITIONS.md](docs/ADDITIONS.md)), each with the upstream checkout as an argument.

Rules for Horizon content: tags only in the namespaces `mekanism`, `minecraft` and `c`. Loot uses Horizon's `condition` object form (`apply_bonus`/`match_tool` are missing). Worldgen in the flat 26.3 format. See [AGENTS-WINDOWS.md](AGENTS-WINDOWS.md).

## Notes

- Warnings like `warning: LF will be replaced by CRLF` on `git add` under Windows are harmless.
- Do not change `README.md` by hand between `<!-- progress:start -->` and `<!-- progress:end -->`. The block is generated from `TODO.md` (`python scripts/sync_readme_progress.py`).
- Paths to the Horizon instance belong only in `scripts/horizon-game-dir.txt` (ignored), never in the repository.
- Report problems: GitHub issue with the Horizon version, native build name, mod versions, reproduction steps and log. Known issues are in [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md).
