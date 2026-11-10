# Known Issues

Stand: 10.11.2026, Version 0.2.0 (Content-Mods `mekanismtools` und `mekanismadditions` 0.1.0). Gesammelt aus den Blocker-Notizen in [TODO.md](../TODO.md), [TOOLS.md](TOOLS.md), [ADDITIONS.md](ADDITIONS.md) und [TESTLOG.md](TESTLOG.md). Details und Begründungen stehen dort. „Horizon-Grenze“ heißt: Der Content-Pfad oder die native Hook-API von Horizon (`fdc87ae`, API 411) bietet es nicht.

## Allgemein

- **Prototyp.** Nur in Testwelten verwenden. Save-Migrationen und ID-Stabilität zwischen Versionen sind nicht zugesichert (M13 offen).
- **Native Mod nötig.** Maschinen, Generatoren, Energie und Tanks laufen nur mit der nativen Mod ([INSTALL.md](INSTALL.md)). Sie läuft ohne Sandbox mit Benutzerrechten.
- **Horizon-Pin.** Getestet nur mit `fdc87ae`. Neuere Stände können die Hook-API ändern.
- **Keine Konfiguration.** Werte (Tiers, Energieverbrauch, Materialwerte, TNT-Radius) sind feste Mekanism-Standardwerte. Horizon-Content hat keine Config.
- **Multiplayer, Mod-Interop (AE2 u. a.), Performance** mit vielen Maschinen und Shader-Kompatibilität sind nicht systematisch getestet.
- **Kein Transport.** Kabel, Pipes, Tubes und Logistik fehlen (M07/M08). Energie fließt nur zwischen benachbarten Blöcken.
- **Fehlende Mod-Fluid-Eimer.** Horizon hat keine Eimer für Mod-Fluids, nur Tanks speichern sie.
- **Native Tests nicht per `cargo test`.** Die Crate `native/mekanism_native/api` hängt an Horizons Client und lässt sich ohne Launcher-Toolchain nicht bauen. CI prüft nur Content-Validierung und `logic/energy`.
- **Zeilenenden-Warnungen** (`LF will be replaced by CRLF`) bei `git add` unter Windows sind harmlos.
- **Inkonsistente Upstream-Referenz.** TODO und `docs/CHEMISTRY.md` nennen Mekanism-Commit `31bb4906`, `docs/TESTLOG.md` nennt `330b7b4d`. Beim nächsten Upstream-Abgleich vereinheitlichen.

## Maschinen und Generatoren

Für Metallurgic Infuser, Enrichment Chamber, Crusher, Osmium Compressor, Combiner, Precision Sawmill, Purification/Injection Chamber, Chemical Oxidizer und Infuser gilt gemeinsam:

- Fehlt: Energie-Slot, Upgrades, Side-Configuration, Redstone-Steuerung.
- Infuse-Inhalt und Energie bleiben beim Abbau nicht am Item (Metallurgic Infuser).
- Rezeptwechsel und Chunk-Unload sind nicht für jede Maschine getestet.
- Balkenfarben genähert, GUI-Layout vereinfacht (176x166 statt Mekanisms größerer Oberfläche beim Chemical Infuser).
- Purification/Injection beziehen Chemie vorerst aus Items (Flint: Sauerstoff, Salz: Chlorwasserstoff). Wasserdampf-Rezepte laufen erst mit einer Maschine, die ihn erzeugt.
- Chemical Oxidizer/Infuser: keine Chemie-Item-Slots, kein Auto-Eject-Schalter (immer alle Seiten), keine Rezeptpaar-Prüfung beim Einfüllen, keine Pfeil-Animation. Crafting-Rezepte (`mek_data` mit Dynamic Tank) fehlen, bis Dynamic Tank/Personal Storage existieren. Test per `/give`.
- Fluid-Outputs (Electrolytic Separator, Pump) und Item-Outputs mit Chemie-Zweitausgabe fehlen im Rezeptsystem.
- Machine-Sounds, Tooltips und Crafting-Rezepte ingame nicht überall geprüft ([TESTLOG.md](TESTLOG.md)).

## Energy Cube

- Seitenweiser Seitenpfad über Horizons `insert_faces` mit echter Quelle nicht getestet (Debug-Blöcke schreiben NBT direkt).
- Neustart-Test der Seiteneinstellung, Crafting-Rezept ingame und Tooltip mit Energie offen.
- Tablet stapelt nicht (`max_stack_size` 1): Test offen.
- Einmaliger, nicht reproduzierbarer Sprung von 252.000 auf 504.000 FE bei Test M01. Vermutlich Item-Energie, keine Duplikation (siehe TESTLOG).

## Tanks und Chemie

- Fluid-Anzeige mit **Shaderpack** (Bliss) falsch dargestellt. Ursache liegt in Horizons Shader-Pfad für `draw_fluid_box`, ohne Shader korrekt.
- Herstellung vieler Fluids/Chemicals fehlt (Electrolytic Separator, Pump, Rotary Condensentrator usw.).
- Volle Tanks, Durchsatzlimits und Weltneustart-Tests sind nicht vollständig.

## Tools (`mekanismtools`)

Siehe Tabelle in [TOOLS.md](TOOLS.md). Kurz:

- **Paxel** ist nur Axt, nicht Schaufel (kein Grasweg). Braucht nativen `item_use_on`-Hook.
- **Paxel-Rezept** startet unbeschädigt, mittelt die Haltbarkeit der Zutaten nicht.
- **Schild** zeigt das Vanilla-Modell statt der Mekanism-Textur.
- **Rüstungs-Trims** ohne Mekanism-Paletten, **kein** Haltbarkeits-Tooltip (`DISPLAY_HP`), keine Mob-Spawn-Ausrüstung, keine Advancements.
- Ausstehende Tests: Schild blocken, Speer, Pferde-/Nautilus-Rüstung, Amboss-Reparatur, Verzauberung, Neustart, Multiplayer.

## Additions (`mekanismadditions`)

Siehe [ADDITIONS.md](ADDITIONS.md). Kurz:

- **Obsidian TNT** explodiert wie Vanilla-TNT (Radius 4 statt 12, feste Zündzeit). Eigene Entität/Explosion fehlt.
- **Plastic Road** ohne Geschwindigkeitsschub. **Glow/Transparent** ohne Emissive/CTM, ohne Face-Auslassung.
- **Kein Lava-Fluid-Logging** bei Fences, Slabs und Obsidian TNT.
- **Fehlende Rezepte:** Plastic-Grundrezept und Glow Panels (brauchen `hdpe_sheet`, M06 Chemie), Umfärben, Painting-Machine, Pigment-Extraktion.
- **Nicht gebaut:** Baby-Mobs, Balloons, Walkie-Talkies, Spawn Eggs (Entitäten und Netzwerk fehlen in Horizon-Content).
- Ingame offen: Crafting im Grid, Slick-Rezept in der Enrichment Chamber, Slick-Reibung, Inventar-Items, Neustart/Chunk-Unload.

## Noch nicht portiert

Multiblocks (Thermal Evaporation, Turbine, Fission/Fusion-Reaktor, Boiler, SPS u. a.), Kabel und Transporter, Digital Miner, Teleporter, Qio-Speicher, Jetpack/Rüstung mit Modulen (Mekanism Core), Meka-Tool. Aktueller Stand: [TODO.md](../TODO.md).

## Fehler melden

Als GitHub-Issue mit Horizon-Stand (Commit/Nightly), Native-Build-Name, Mod-Versionen, Schritten zur Reproduktion und Log-Auszug. Siehe [CONTRIBUTING.md](../CONTRIBUTING.md).
