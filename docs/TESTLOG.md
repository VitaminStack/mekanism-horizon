# Ingame-Testprotokoll

Horizon Nightly 2026-11-08 `fdc87ae`, Native-Hooks-API 411, Mekanism-Basis 26.3 (`330b7b4d`).
Build: `scripts/Build-Content.ps1` (Content), `scripts/Build-Native.ps1 -TrustNativeCode` (Native).

## 09.11.2026

| Test | Ergebnis |
|---|---|
| Native-Build mit Vertrauensfreigabe, Installation im Launcher | ok |
| `/give` + Platzieren aller 5 Energy Cubes | ok |
| Cube-Drop mit Netherite-Spitzhacke (Loot-Format `condition`) | ok |
| Osmium-Drops, Schmelzen, Barren zu Block | ok |
| Erz- und Salzgenerierung (Flat-Format 26.3, Biome-Tag `spawn_ores`) | ok |
| Block-Entity je Cube (Agent Control `/data`) | ok |
| Creative Cube füllt sich auf `i32::MAX` | ok |
| `energy: 123456` überlebt Speichern + Neustart | ok |
| Energy Tablet speichert Energie | ok |
| Tablet stapelt nicht (`max_stack_size` 1) | ausstehend |
| Debug-Quelle füllt Basic Cube: 4.000/Tick, stoppt bei 4.000.000 (Agent Control) | ok |
| Debug-Senke leert Cube: 4.000/Tick bis 0 | ok |
| Seitenweise Ein-/Ausgabe über Horizons Energie-Pfad | ausstehend (braucht echte Quelle, M04/M05) |

Screenshots: wurden im Chat geteilt, nicht im Repo abgelegt.
