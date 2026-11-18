# Installation Guide (Windows)

Mekanism Horizon consists of **content mods** (`.hmod`, data only) and a **native Rust mod** (`.hnmod`, machine logic). Without the native mod, items, blocks, recipes and world generation load, but machines, generators, energy and tanks do nothing.

> Experimental prototype. Use it only in test worlds and back up worlds first. Known limitations: [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

## Horizon version

Everything is built and tested against **Horizon Nightly 2026-11-08, commit [`fdc87ae`](https://github.com/horizon-rust/horizon/commit/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca)** (native hooks API 411). The pin appears in two places and must match:

- `native/mekanism_native/horizon-mixin.toml` (`horizon_revision`)
- `native/mekanism_native/api/Cargo.toml` (`rev`)

Other Horizon builds may run but are not tested. The native mod depends on the hook API, so a version change may need code changes.

## Requirements

| What | Used for | Check |
|---|---|---|
| [Horizon Launcher](https://github.com/horizon-rust/horizon) with an instance | Starting the game, native build | `horizon-launcher` in the launcher folder |
| Rust (MSVC toolchain, [rustup](https://rustup.rs/)) | Content build, logic tests | `cargo --version` |
| `cargo-horizon-mod` | Build and check content | `scripts/Install-Horizon-Tool.ps1` |
| Python 3.11+ | Content validation, generators | `python --version` |
| Git | Clone the repository | `git --version` |

The native build uses the **toolchain managed by the launcher**, not your own. Visual Studio 2026 is optional.

## 1. Repository and game folder

```powershell
git clone https://github.com/VitaminStack/mekanism-horizon.git
cd .\mekanism-horizon
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Install-Horizon-Tool.ps1"
```

In the launcher, select the instance and open its **game folder** (it contains `options.txt`). Write the path into the local file `scripts/horizon-game-dir.txt`, one line, for example `C:\Pfad\zur\Horizon-Instanz`. Do not give `horizon-mods`, but the parent folder. The file is in `.gitignore`.

## 2. Build and install content

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Test-Content.ps1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

`Build-Content.ps1` builds four content mods in turn, checks each package with `cargo horizon-mod check` and copies it to `<game folder>\horizon-mods\`:

| Mod | Content | Needs |
|---|---|---|
| `mekanism` | Core: ores, machines, Energy Cubes, chemistry | native |
| `mekanismgenerators` | Generators | `mekanism`, native |
| `mekanismtools` | Tools, armor, Paxel | `mekanism` |
| `mekanismadditions` | Plastic blocks, Glow Panels, Obsidian TNT | content only |

New mods start **off**. Enable them in `<game folder>\horizon-mods.txt` or in the launcher's mod list (line `mod.<id>:true`). Restart the game after registry changes.

The first start after a build is slow (content cache, about 46 s in the test), the second is fast.

## 3. Build the native mod (trust approval)

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

### What "trust native code" means

Horizon compiles the Rust code in `native/mekanism_native/` into its own game version and runs it **inside the game process with your Windows user permissions**. It is not sandboxed: it could read and write files or open network connections. Therefore:

- The `-TrustNativeCode` switch is required. Without it, the script aborts.
- After that, the script asks you to type `BUILD`. `-Confirmed` skips this question. Use it only if you have checked the source code yourself.
- Check the code (at least `native/mekanism_native/api/src/`) before the first build and after every pull. Build only versions from sources you trust, including foreign pull requests.

The first full build took about 11 minutes on the test system. Later builds are faster. After success, the script selects the newest `native-*` version for the instance `horizon` and removes older ones (`-CleanupOnly` does only that).

Launcher not found? Give the path:

```powershell
$env:HORIZON_LAUNCHER = "F:\Games\Horizon Launcher\horizon-launcher.exe"
```

or `-Launcher <path>`. Search order: parameter/`HORIZON_LAUNCHER`, `PATH`, `%LOCALAPPDATA%\Programs\Horizon Launcher`, `F:\Games\Horizon Launcher`.

## 4. Start

Start Horizon through the launcher with the newly built `native-*` version, enable the content mods, open a test world.

```mcfunction
/give @s mekanism:basic_energy_cube 1
/give @s mekanism:enrichment_chamber 1
/give @s mekanism:osmium_ingot 64
```

## Common errors

| Message | Solution |
|---|---|
| `Horizon game folder missing` / `options.txt missing` | Create `scripts/horizon-game-dir.txt` or set it to the game folder instead of `horizon-mods` |
| `Cargo is missing` | Install Rust, restart PowerShell |
| `Horizon Launcher not found` | Set `HORIZON_LAUNCHER` or `-Launcher` |
| `Native code needs your explicit approval` | Add `-TrustNativeCode` (see above) |
| PowerShell blocks scripts | Start with `powershell.exe -NoProfile -ExecutionPolicy Bypass -File ...` |
| New mod missing in the game | Enable the mod in `horizon-mods.txt`, restart the game |
| Machines can be placed but do nothing | Native version not selected or not built |
| `... not removed (is Horizon still running?)` | Horizon is still running, close it and run `-CleanupOnly` |

More Windows details: [README-WINDOWS.md](../README-WINDOWS.md). Test results: [TESTLOG.md](TESTLOG.md).
