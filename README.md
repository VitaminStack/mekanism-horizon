# Mekanism Horizon

Unofficial port of [Mekanism](https://github.com/mekanism/Mekanism) for [Minecraft Horizon](https://github.com/horizon-rust/horizon).

**v0.2.0 · Experimental prototype**

## Porting progress

<!-- progress:start -->
```text
Total                       ███████████░░░░░░░░░  54%  (90/166)

Project base                ████████████████████ 100%  (10/10)
Energy Cube                 ████████████████████ 100%  (10/10)
Resources & energy          █████████████░░░░░░░  65%  (15/23)
Machines & chemistry        ██████████████░░░░░░  70%  (19/27)
Automation & logistics      ████░░░░░░░░░░░░░░░░  20%  (4/20)
Multiblocks & reactors      ████░░░░░░░░░░░░░░░░  19%  (3/16)
Tools & Additions           ██████████░░░░░░░░░░  52%  (16/31)
Tests & releases            ███░░░░░░░░░░░░░░░░░  16%  (3/19)
Next steps                  ████████████████████ 100%  (10/10)
```
<!-- progress:end -->

[Full roadmap](TODO.md) · Calculated from the checked tasks in `TODO.md`. The bars update automatically when TODO.md changes.

## Project

- [Content mod](content/mekanism/) – items, blocks, models and recipes (`.hmod`)
- [Native mod](native/mekanism_native/) – block entities, machines and energy hooks (Rust)
- [Energy tests](logic/energy/) – standalone Rust unit tests

## Getting started on Windows

```powershell
cargo test --manifest-path .\logic\energy\Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

Requirements, game folder configuration and the separate native build: **[Windows guide](README-WINDOWS.md)**.

> Tested in game: resources and ores, Energy Cubes and Energy Tablet, Metallurgic Infuser, Enrichment Chamber, Crusher, Osmium Compressor, Combiner, Precision Sawmill, Purification and Chemical Injection Chamber, Chemical Oxidizer and Infuser, Electrolytic Separator, Rotary Condensentrator, Dissolution Chamber, Washer and Crystallizer (ore chain), Heat, Solar, Advanced Solar, Bio, Wind and Gas-Burning Generator, Fluid and Chemical Tanks, Mechanical Pipes, Pressurized Tubes and Thermodynamic Conductors. Wave 1 added Oredictionificator, Digital Miner, Teleporter, Quantum Entangloporter (items and energy), Dynamic Tank, Induction Matrix, Flamethrower, Free Runners and Scuba gear (`docs/TESTLOG.md`). Use in test worlds only.

[Installation](docs/INSTALL.md) · [Changelog](CHANGELOG.md) · [Known Issues](docs/KNOWN_ISSUES.md) · [Contributing](CONTRIBUTING.md) · [Licenses](docs/LICENSES.md)

[Technical details](docs/ENERGY-CUBE.md) · [Task list](TODO.md) · [License](LICENSE)

*Unofficial project, not affiliated with Mekanism or Horizon.*
