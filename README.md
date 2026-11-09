# Mekanism Horizon

Inoffizieller Port von [Mekanism](https://github.com/mekanism/Mekanism) für [Minecraft Horizon](https://github.com/horizon-rust/horizon).

**v0.2.0 · Experimenteller Prototyp**

## Portierungsfortschritt

<!-- progress:start -->
```text
Gesamt                      ██░░░░░░░░░░░░░░░░░░   9%  (14/150)

Projektbasis                ████████████░░░░░░░░  60%  (6/10)
Energy Cube                 ░░░░░░░░░░░░░░░░░░░░   0%  (0/10)
Rohstoffe & Energie         ███░░░░░░░░░░░░░░░░░  17%  (4/23)
Maschinen & Chemie          ░░░░░░░░░░░░░░░░░░░░   0%  (0/26)
Automation & Logistik       ░░░░░░░░░░░░░░░░░░░░   0%  (0/17)
Multiblocks & Reaktoren     ░░░░░░░░░░░░░░░░░░░░   0%  (0/16)
Tools & Additions           ░░░░░░░░░░░░░░░░░░░░   0%  (0/19)
Tests & Releases            ░░░░░░░░░░░░░░░░░░░░   0%  (0/19)
Naechste Schritte           ████████░░░░░░░░░░░░  40%  (4/10)
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

> Osmium und der platzierbare Basic Energy Cube wurden im Spiel getestet. Der native Rust-Build war erfolgreich; **Energiespeicherung und -übertragung sind noch nicht im Spiel bestätigt**. Nur in Testwelten verwenden.

[Technische Details](docs/ENERGY-CUBE.md) · [Aufgabenliste](TODO.md) · [Lizenz](LICENSE)

*Inoffizielles Projekt, nicht mit Mekanism oder Horizon verbunden.*
