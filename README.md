# Mekanism Horizon

Inoffizieller Port von [Mekanism](https://github.com/mekanism/Mekanism) für [Minecraft Horizon](https://github.com/horizon-rust/horizon).

**v0.2.0 · Experimenteller Prototyp**

## Portierungsfortschritt

<!-- progress:start -->
```text
Gesamt                      ███████░░░░░░░░░░░░░  37%  (56/151)

Projektbasis                ████████████████████ 100%  (10/10)
Energy Cube                 ████████████████████ 100%  (10/10)
Rohstoffe & Energie         ████████████░░░░░░░░  61%  (14/23)
Maschinen & Chemie          ██████░░░░░░░░░░░░░░  31%  (8/26)
Automation & Logistik       ░░░░░░░░░░░░░░░░░░░░   0%  (0/17)
Multiblocks & Reaktoren     ░░░░░░░░░░░░░░░░░░░░   0%  (0/16)
Tools & Additions           ████░░░░░░░░░░░░░░░░  20%  (4/20)
Tests & Releases            ░░░░░░░░░░░░░░░░░░░░   0%  (0/19)
Naechste Schritte           ████████████████████ 100%  (10/10)
```
<!-- progress:end -->

[Zur vollständigen Roadmap](TODO.md) · Berechnet aus den abgehakten Aufgaben in `TODO.md`. Die Balken werden bei TODO-Änderungen automatisch aktualisiert.

## Projekt

- [Content-Mod](content/mekanism/) – Items, Blöcke, Modelle und Rezepte (`.hmod`)
- [Native-Mod](native/mekanism_native/) – Block-Entity und Energie-Hooks (Rust)
- [Energie-Tests](logic/energy/) – unabhängige Rust-Unit-Tests

## Start unter Windows

```powershell
cargo test --manifest-path .\logic\energy\Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

Voraussetzungen, Spielordner-Konfiguration und separater Native-Build: **[Windows-Anleitung](README-WINDOWS.md)**.

> Im Spiel getestet: Rohstoffe und Erze, Energy Cubes und Energy Tablet, Metallurgic Infuser, Enrichment Chamber, Crusher, Osmium Compressor, Combiner, Precision Sawmill, Purification und Chemical Injection Chamber, Heat-, Solar-, Advanced-Solar-, Bio- und Wind-Generator sowie Fluid und Chemical Tanks (`docs/TESTLOG.md`). Kabel, Pipes und die meisten Maschinen fehlen noch. Nur in Testwelten verwenden.

[Technische Details](docs/ENERGY-CUBE.md) · [Aufgabenliste](TODO.md) · [Lizenz](LICENSE)

*Inoffizielles Projekt, nicht mit Mekanism oder Horizon verbunden.*
