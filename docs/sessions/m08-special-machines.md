# M08 – Spezialmaschinen (Session-Doku)

Branch `claude/m08-special-machines-fb6601`. Upstream-Referenz: Mekanism 26.3 (`src/main/java/mekanism/common/...`).
Mekanism ist MIT-lizenziert; der Hinweis bleibt erhalten.

**Wichtig:** Der Native-Build war in dieser Session nicht erlaubt. Nur die Horizon-freien Module wurden
kompiliert und getestet (`cargo test --manifest-path logic/energy/Cargo.toml`, 149 Tests ok).
Alle Host-Module sind **nicht compile-verifiziert** (siehe unten).

## Commits

| Commit | Inhalt |
|---|---|
| `7c3da58` | Oredictionificator |
| `0bb896d` | Formulaic Assemblicator + Crafting Formula (nur Registrierung) |
| `dc3ac70` | Digital Miner |

Seismic Vibrator und Seismic Reader (Aufgabe 4, optional) wurden **nicht** begonnen.

## 1. Oredictionificator

Module: `oredict.rs` (Horizon-frei, 7 Tests), `oredicts.rs` (Host), `oredict_tags.rs` (generiert aus
`scripts/gen_oredict_tags.py`), Router `special.rs`.

| Wert | Quelle |
|---|---|
| Kein Energiebedarf | `TileEntityOredictionificator` |
| Slots Eingang (56,115), Ausgang (164,115); GUI 236x230, Inventar bei (38,148) | `GuiOredictionificator` |
| Erlaubte Tag-Präfixe `c:` + `ingots/ ores/ dusts/ nuggets/ storage_blocks/ raw_materials/` | `GeneralConfig.java:229-230` |
| Pro Tick werden min(Anzahl, freier Platz) umgewandelt | `TileEntityOredictionificator.tickServer` |

Abweichungen:
- Max. 8 Filter, 3 sichtbar; Tag und Ausgabe werden per Zyklus-Buttons gewählt (Horizon hat keine Texteingabe im Screen).
- Eingangs-Match hybrid: generierte Tag-Tabelle ODER `server_item_in_tag` (siehe Blocker 1).
- Tags anderer Mods unter `c:` sind nicht filterbar, wenn sie nicht in der Tabelle stehen.
- Nicht portiert: Redstone-Modus, Seitenkonfiguration, Security, Upgrades.

## 2. Formulaic Assemblicator + Crafting Formula

Nur registriert: Block, Item, Rezepte, Modelle, Lang. **Kein Verhalten** (Blocker 2).

## 3. Digital Miner

Module: `miner.rs` (Horizon-frei, 10 Tests), `miners.rs` (Host), Verdrahtung in `special.rs`, `energy.rs`, `lib.rs`.

| Wert | Quelle |
|---|---|
| Standard: Radius 10, minY 0, maxY 60 | `TileEntityDigitalMiner` |
| maxRadius 32, silkMultiplier 12, ticksPerMine 80 | `GeneralConfig` |
| Basisverbrauch 40 FE/Tick | `UsageConfig` (digitalMiner) |
| Verbrauch/Tick: `ceil(base * (silk?12:1) * (1+max(0,(r-10)/22)) * (1+max(0,(maxY-minY-60)/(weltHöhe-1-60))))` | `MinerEnergyContainer.updateMinerEnergyPerTick` |
| Suchreihenfolge: x schnellster, dann z, dann y; Startecke `(x-r, minY, z-r)` | `ThreadMinerSearch` / `TileEntityDigitalMiner` |
| Abbau: nach Chunk in Fundreihenfolge, höchster Index zuerst; ein Block je 80 Ticks | `TileEntityDigitalMiner.tickServer` |
| Auswurf alle 10 Ticks, durch die Zelle oben-hinten in den Behälter dahinter | `TileEntityDigitalMiner` |
| 27 Slots bei (8+x*18, 92+y*18) | `GuiDigitalMiner` |
| Fußabdruck x/z -1..1, y 0..1 (17 Bounding-Blöcke) | `BlockDigitalMiner` / `Attributes` |
| Radius 0..32; minY in [Weltmin, maxY]; maxY in [minY, Weltmax] | `TileEntityDigitalMiner.setRadius/setMinY/setMaxY` |
| `miner_blacklist`-Tag ist upstream leer | Datenpack |

Abweichungen:
- Statischer Puffer 100 000 FE (upstream dynamisch, 4 x Verbrauch).
- Filter = Items in 9 Filter-Slots + eine Tag-Auswahl (8 Block-Tags), keine Filter-GUI mit Replace-Target/Ausgabe.
- Werkzeug zum Abbauen: Netherite-Spitzhacke (Silk Touch bei aktivem Silk), upstream voll geladener Atomic Disassembler.
- Suchzustand und Overflow werden nicht gespeichert; nach Neustart wird neu gesucht.
- Einstellungen sind gesperrt, solange der Miner läuft.
- Hopper ziehen oben und seitlich.
- Nicht portiert: Redstone, Seitenkonfiguration, Security, Upgrades, XP, Anker, Filter auf dem gedroppten Item.
- Unzerstörbare Blöcke per Liste (keine Härte-API).
- Rechtsklick auf einen Bounding-Block des Miners wird abgebrochen (kein Aufruf zum Öffnen des Menüs eines anderen Blocks).
- Blockstate: `mekanism:holiday_selector` entfernt (Horizon kennt den Typ nicht); `active` ist eine Boolean-Property und wird in `miners::tick` nur bei Änderung gesetzt.

## Blocker (aus dem Quelltext belegt)

1. **Keine Tag-Auflistung im Host.** Nur Mitgliedschaftstests (`server_item_in_tag`, `block_name_in_tag`). Daher generierte Tabelle.
2. **Vanilla-Crafting-Rezepte nicht erreichbar.** `native_recipes_of_type` liefert nur `recipes().custom.get(kind)`
   (`crates/horizon-client/src/game/native_hooks.rs:11288-11293`). `custom_type` (`crafting.rs:1333-1337`) behält nur Typen
   mit `:` und ohne Präfix `minecraft:`. Der Loader (`crafting.rs:1546-1559`) leitet `minecraft:crafting_*` in `crafting`,
   das nicht exponiert ist. `native_recipes` (`native_hooks.rs:11359`) ist nicht im Host-Trait; `docs/native-mods.md:4999-5001`
   führt `recipes` unter "noch nicht im Trait". Der Formulaic Assemblicator kann also keine Rezepte auflösen.
3. Screens kennen nur `host.text` und `host.fill`, keine Stringübergabe; Namen werden aus Compile-Time-Tabellen dekodiert.
4. Screen-Hooks erhalten kein `pos`; GUI-Zustand wird als `i32`-Daten (16-Bit-Wire) kodiert, 32-Bit-Werte über `cube::encode_amount`.

## Nicht compile-verifiziert

Nur Horizon-freie Logik wurde kompiliert/getestet. Nicht kompiliert:
- `native/mekanism_native/api/src/oredicts.rs`
- `native/mekanism_native/api/src/miners.rs`
- `native/mekanism_native/api/src/special.rs`
- Neue Arme in `lib.rs` (`block_entity_container`, `container_accepts`, `block_entity_energy`, `block_entity_tick`, `state_for_placement`, `screen`, `screen_draw`, `screen_click`, `menu_button`, `right_click_block`, `destroy_block`, `place_block`)
- Erweiterung `acceptor()` in `energy.rs`
- Das generierte `oredict_tags.rs` (nur als Horizon-freies Modul in `logic/energy` kompiliert)

## Ausgeführte Prüfungen

- `python scripts/validate_content.py`: OK (5653 JSON-Dateien)
- `cargo test --manifest-path logic/energy/Cargo.toml`: 149 passed
- `python scripts/parity.py --check`: 0 Regressionen
- `cargo horizon-mod build` + `check` in `content/mekanism`: ok (`mekanism 0.2.0`, API 0.2, content only)

## Ingame-Testliste

Oredictionificator:
1. Platzieren, Menü öffnet (236x230). Inventar sitzt unter dem Gerät.
2. Filter anlegen, Tag und Ausgabe durchschalten, Scroll bei >3 Filtern.
3. Eisenbarren-Ingot-Tag: Eingang `minecraft:iron_ingot` wird zum gewählten Ausgabeitem; Stackgröße bleibt, wird pro Tick begrenzt durch Platz.
4. Hopper schiebt nur passende Items ein. Deaktivierter Filter wandelt nicht.
5. Filter überleben Neustart/Chunk-Reload.

Formulaic Assemblicator: Block und Crafting Formula erscheinen im Kreativ-Inventar mit Modell und Namen. Kein Verhalten erwartet.

Digital Miner:
1. Platzieren: 17 Bounding-Blöcke entstehen, Platzieren bei Hindernis wird abgelehnt. Abbau des Hauptblocks und eines Bounding-Blocks entfernt alle.
2. Menü öffnen (Hauptblock). Radius/minY/maxY per Buttons ändern, Grenzen 0..32 und Weltgrenzen.
3. Energie per Kabel/Cube einspeisen; Verbrauch pro Tick prüfen (Radius 10, Höhe 60: 40 FE/Tick; Silk: 480 FE/Tick; Radius 32: 40*(1+22/22)=80).
4. Start: Suche läuft, danach ein Block je 80 Ticks, höchster Index zuerst; Modell wechselt auf `active`.
5. Filter-Slot mit Item bzw. Tag-Auswahl: nur passende Blöcke werden abgebaut (bzw. invers).
6. Silk Touch: Erzblock droppt sich selbst.
7. Auswurf: Behälter hinter dem Miner, oben-hinten Zelle, alle 10 Ticks.
8. Volles Inventar: Overflow blockiert den Abbau, danach Weiterlauf.
9. Einstellungen sind gesperrt, solange er läuft.
10. Welt neu laden: Suche beginnt neu, Einstellungen und Filter bleiben.

## Offene Punkte

- Native-Build und Ingame-Test stehen aus; erst danach ist Host-Code verifiziert.
- Formulaic Assemblicator braucht eine Host-API für Crafting-Rezepte.
- Seismic Vibrator und Seismic Reader fehlen.
- Dynamischer Energiepuffer, Filter-GUI mit Replace-Target, Upgrades, Redstone, Seitenkonfiguration fehlen.
