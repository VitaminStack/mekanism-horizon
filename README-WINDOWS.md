# Mekanism Horizon – Windows development guide

Tested setup: **Windows / Visual Studio 2026**, **Horizon Nightly `fdc87ae`**, content mod **v0.2.0**, native Rust companion (build `native-fdc87ae-8abc9345573f`). Energy storage checked in game, see the [test log](docs/TESTLOG.md).

## Requirements

- [Horizon Launcher](https://github.com/horizon-rust/horizon) with an installed game instance.
- Rust and Cargo: check with `cargo --version` and `rustc --version`.
- Horizon content CLI `cargo horizon-mod` (set it up with `scripts/Install-Horizon-Tool.ps1`).
- Windows PowerShell and, optionally, Visual Studio 2026 (local `Ctrl+B` configuration).

The native Horizon build uses a **Rust toolchain managed by the launcher** and creates its own game version. The content mod (`.hmod`) contains no native Rust code.

## Check out the project and configure the instance

```powershell
git clone https://github.com/VitaminStack/mekanism-horizon.git
cd .\mekanism-horizon
```

In the Horizon Launcher, select the instance you want and open its **game folder**. It contains `options.txt`. Enter this path in the text file `scripts/horizon-game-dir.txt`, which you create locally, for example:

```text
C:\Path\to\Horizon-Instance
```

Do **not** give `horizon-mods`. Give its **parent folder**. Git ignores this file, and it deliberately holds no generic path.

## Test, build and install the content mod

Run the following commands in the root folder of the repository:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Test-Content.ps1"
cargo test --manifest-path .\logic\energy\Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

The build creates `content\mekanism\target\horizon-mod\mekanism.hmod`, checks the file with `cargo horizon-mod check` and installs it to `<game folder>\horizon-mods\mekanism.hmod`.

**Content cache:** The current script copies the file again on every build, even if the content stays the same. This can make Horizon rebuild its content cache. In the local test, a first start took about 46 seconds; the second start without another build was fast. The planned optimization is a SHA-256 comparison before copying.

## Build the native Rust mod

The native companion lives in `native/mekanism_native/`. The code declares a block entity and an energy capability. It runs **with the rights of the signed-in user**. So review the code first, then approve it explicitly:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

The script also asks you to confirm with `BUILD`. The native mod is compiled into its own game version through `horizon-launcher native-build`. The first full build took **10 minutes 50 seconds** on the tested system; later builds can be faster.

### Portable launcher on Windows

If `Horizon Launcher not found` appears, `horizon-launcher.exe` is probably not in `PATH` or at the default install location. Example for a **portable install** (adjust the path to your system):

```powershell
$env:PATH = "F:\Games\Horizon Launcher;$env:PATH"
Get-Command horizon-launcher.exe
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

The `PATH` entry applies only to this PowerShell window.

## Test in game

Start Horizon **through the launcher** with the matching instance and the newly built native game version. Enable the content mod; restart the game after registry changes. Use only a test world or a world you backed up first.

```mcfunction
/give @s mekanism:osmium_ingot 64
/give @s mekanism:osmium_block 64
/give @s mekanism:basic_energy_cube 1
```

Test energy: place `mekanism:debug_energy_source` or `mekanism:debug_energy_sink` next to a cube (4,000/tick) and read the fill level with `/data get block <x> <y> <z>`.

**Confirmed:** block entity per cube, insert/extract, limits, empty/full, saving and restart. Results in [docs/TESTLOG.md](docs/TESTLOG.md).

**Still open:** per-side input/output through Horizon's energy path (needs a real source). The cube has **no GUI**.

## CI

`.github/workflows/ci.yml` runs on push to `main` and on pull requests: `scripts/validate_content.py` (JSON syntax, manifest) and `cargo test` for `logic/energy`. The native build does **not** run in CI; it needs the Horizon Launcher.

## Common errors

| Message | Fix |
| --- | --- |
| `options.txt missing` | Check that `horizon-game-dir.txt` points to the **game folder**, not to `horizon-mods`. |
| `Horizon Launcher not found` | Add the path to the launcher EXE to `PATH` temporarily, as above. |
| PowerShell blocks scripts | Use `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <script>`. |
| First game start is slow | Probably the content cache was rebuilt after the last build; check a second start without another build. |
| Cube placed, but no GUI | Expected in v0.2.0; the GUI and full energy function are not implemented. |
| `git apply` reports files that already exist | The patch may already be applied: first check `git status` and `git apply --reverse --check <patch>`. |

## More information

- [Project overview](README.md)
- [Energy Cube technical notes](docs/ENERGY-CUBE.md)
- [Porting roadmap](TODO.md)
- [Horizon native mod documentation](https://github.com/horizon-rust/horizon/blob/main/docs/native-mods.md)

**License note:** Unofficial port. Keep the rights and copyright notices of the [original Mekanism project](https://github.com/mekanism/Mekanism) whenever you later take over code or assets. See [LICENSE](LICENSE).
