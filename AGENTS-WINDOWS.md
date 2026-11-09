# Coding agent notes for this Windows starter

- The repo root contains the older Linux-validated native mixin template. Do not silently replace it.
- Content source is in `content/mekanism/` (Horizon content API 0.2). Run `cargo horizon-mod build` **there**, never at repository root.
- Run `python scripts/validate_content.py` after every content change, then official `cargo horizon-mod check` when the tool is available.
- `logic/energy/` is an independent Rust library for tests only, not wired to game ticks, block entities or persistence.
- Do not claim machines, pipes or world generation are implemented. Prefer small testable additions.
- Native mod development: use `cargo horizon-mod new ... --template native` and the Horizon launcher on Windows, using current hook contracts.
- Do not copy Mekanism assets/code without preserving applicable licensing and attribution.
- Resource content (M02) is generated, not hand-edited: `python scripts/gen_resources.py`, then `python -I scripts/import_upstream.py <Mekanism 26.3 checkout>` (sparse clone is enough: `src/datagen/generated/mekanism`, `src/main/resources/assets/mekanism/{textures,models,lang,blockstates,items}`, `LICENSE`). Then `scripts/Build-Content.ps1`.
- Horizon content mods may add tags only in `mekanism`, `minecraft` and `c`. Loot pools use Horizon's `condition` object form; `apply_bonus`/`match_tool` are unsupported.
- Worldgen features use the 26.3 flat format in `data/mekanism/worldgen/feature/` (no `config` wrapper; `state` is a block id string). Biome tags (`tags/worldgen/biome`) must be written by `gen_resources.py`; the importer copies only block/item tags.
- Launcher may keep an old native build selected: `scripts/Build-Native.ps1` (`-Confirmed`, `-CleanupOnly`) sets instance `horizon` to the newest `native-*` and removes older ones. Agent Control: `%APPDATA%\horizon-launcher\versions\<native>\horizon-ctl.exe state|command|setblock|quit`; wait for `player.block_pos` before world commands (earlier ones crash Horizon).
