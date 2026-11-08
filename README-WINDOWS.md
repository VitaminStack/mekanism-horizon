# Mekanism Horizon – Windows-Starter (unoffiziell)

Diese Dateien **ergänzen** das bereits vorhandene Repository
`VitaminStack/mekanism-horizon` (Horizon Native Mod Template).
Den vorhandenen Root-Ordner `mixins/` und die Root-`Cargo.toml` nicht löschen.
Der Root-Template-Build ist nicht als Windows-Staging-Prozess validiert;
dieses Paket umgeht ihn fuer die ersten Content-Schritte.

## Status / ehrlich abgegrenzt

- Fertig: Content-Mod `mekanism` (API 0.2), Osmiumbarren und Osmiumblock.
- Provisorisch: Beide benutzen **Vanilla-Eisen-Texturen als Platzhalter**.
- Fertig: Offline-Strukturtest und Windows-PowerShell-Buildskripte.
- Vorbereitung: Separates, testbares `logic/energy` mit Rust-Unit-Tests.
- Noch **nicht** fertig: Maschinen, GUIs, Energienetzwerke, Weltgenerierung,
  echte Mekanism-Assets und die Anbindung der Energie-Logik ans Spiel.
- Dieses Paket wurde nicht mit `cargo horizon-mod check` oder dem Spiel
  getestet. Die offizielle Validierung bitte lokal ausfuehren.

## Installation auf Windows 10/11 (Visual Studio 2026)

1. Rust fuer Windows (MSVC) von https://rustup.rs installieren.
   In Visual Studio sollte `Desktopentwicklung mit C++` installiert sein,
   falls der Linker fehlt. Danach Developer PowerShell **neu starten**.
2. `cargo --version` und `rustc --version` pruefen.
   Im Repo liegt eine `rust-toolchain.toml`, die Rust 1.95.0 anfordert.
3. Im Projekt-Stammverzeichnis die offiziellen Horizon-Werkzeuge installieren:

   ```powershell
   .\scripts\Install-Horizon-Tool.ps1
   ```

4. Mod erstellen und offiziell pruefen:

   ```powershell
   .\scripts\Test-Content.ps1
   .\scripts\Build-Content.ps1
   ```

   Ergebnis: `content\mekanism\target\horizon-mod\mekanism.hmod`.

Alternativ ohne Cargo: Die hier beigelegte `mekanism-preview.hmod` kann zum
lokalen Spielen ausprobiert werden. Sie ist als **Vorschau** manuell als ZIP
gepackt, nicht mit der Horizon-CLI validiert.

## In Horizon spielen

- In den Spielordner mit `options.txt` gehen; `horizon-mods` erstellen.
- `mekanism.hmod` aus dem Build dort hinein kopieren (oder die Vorschau testen).
- Horizon starten, unter `Optionen > Mods` die Mod einschalten und neu starten.
- In einer Testwelt `mekanism:osmium_ingot` und `mekanism:osmium_block`
  im Kreativ-Inventar suchen oder per `/give` holen.
- Teste: 9 Barren -> 1 Block; 1 Block -> 9 Barren; Blockabbau mit Spitzhacke.

**Alternative:** Im Horizon-Launcher unter Einstellungen als Game-Argument
`--dev-mod "F:\\...\\mekanism-horizon\\content\\mekanism"` angeben,
dann das Spiel starten. Der Pfad ist nur ein Beispiel.

## Energie auf Windows testen

```powershell
cargo test --manifest-path .\logic\energy\Cargo.toml
```

Das ist vorerst **nur** eine standalone Logik-Bibliothek, keine funktionierende
Horizon-Maschine. Nächster Schritt: Block-Entity-Hooks und Save/Load-Abbildung.

## Native-Rust-Mod spaeter auf Windows vorbereiten

```powershell
.\scripts\New-Native-Template.ps1
```

Dieses Skript ruft den **aktuellen offiziellen** Generator auf:
`cargo horizon-mod new <pfad> --template native`. Nach einer Implementierung
kompiliert der Launcher die Native-Mod mit
`horizon-launcher native-build <pfad> --trust-native-code`.
Der vollstaendige Staging-/Check-Workflow aus dem alten
`horizon-native-mod-template` nutzt Linux-Werkzeuge; fuer den Windows-Einstieg
sind diese Schritte nicht erforderlich. Keine Aussage ueber gepruefte
In-Game-Kompatibilitaet von Native-Mods, bevor der Launcher erfolgreich baut.

## Weitere Hinweise

- Mekanism-Original: https://github.com/mekanism/Mekanism
- Horizon Content-API: https://playhorizon.dev/docs/content-mods
- Horizon Native-Mods: https://playhorizon.dev/docs/make-a-mod
- Die Basis ist **inoffiziell**; keine Original-Assets oder Java-Dateien
  wurden uebernommen. Vor spaeterer Uebernahme originaler Assets und Codes
  Copyright-/Lizenzhinweise des jeweiligen Upstream-Projekts beachten.
- `.vs/` und untergeordnete `target/`-Ordner werden ignoriert.
