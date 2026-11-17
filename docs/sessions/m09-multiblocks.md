# M09 – Multiblocks (Session `m09-multiblocks`)

Branch `claude/m09-multiblocks-640bbc`, Basis `main` @ 6a6da54. Ein Commit pro Schritt, Präfix `feat(m09):`.

## Was portiert wurde

1. Allgemeines Multiblock-System (reine Logik + Host).
2. Dynamic Tank, Dynamic Valve, Structural Glass.
3. Induction Matrix: Casing, Port, vier Zellstufen, vier Providerstufen.
4. Thermal Evaporation Plant: **nicht portiert** (nur Validator-Regeln, siehe Lücken).

Boiler und Turbine bleiben unberührt (laut Auftrag).

### Dateien

| Datei | Inhalt |
|---|---|
| `native/mekanism_native/api/src/multiblock.rs` | Reine Logik: Flood-Fill, Bounding Box, Regeln pro Position, `merge_stored` (10 Tests) |
| `native/mekanism_native/api/src/dynamic_tank.rs` | Reine Logik: Kapazität, Valve-Austausch, Edit-Modi, Menü-Daten (10 Tests) |
| `native/mekanism_native/api/src/induction.rs` | Reine Logik: Zellen/Provider-Summen, Port-Fluss, Menü-Daten (10 Tests) |
| `native/mekanism_native/api/src/multiblocks.rs` | Host: Bildung, Verknüpfung, Revalidierung, Weiterleitung der Hooks |
| `native/mekanism_native/api/src/dynamic_tanks.rs` | Host: Tank-Inhalt in NBT, Valve ↔ Master, Screen |
| `native/mekanism_native/api/src/inductions.rs` | Host: Matrix-Energie in NBT, Port ↔ Master, Screen |
| `native/mekanism_native/api/src/lib.rs` | Nur additiv: `Kind::Multiblock`, Match-Arme, `place_block`/`destroy_block` |
| `logic/energy/src/lib.rs` | Drei neue `#[path]`-Module für die Tests |
| `scripts/gen_resources.py` | Additiv: Block "multiblocks (M09)"; generierte Inhalte (Blockstates, Items, Modelle, Texturen, Lang, Rezepte, Loot, Menüs, Tags, `registry.json`) |

## Werte und ihre Quelle (Upstream-Java)

Upstream-Checkout: `mekanism-upstream-26.3`.

| Wert | Quelle |
|---|---|
| Dynamic Tank Kapazität = Volumen (volle Bounding Box) × 16 000 000 mB (`16_000L * BUCKET_VOLUME`) | `GeneralConfig.dynamicTankFluidPerTank`, `TankMultiblockData` |
| Auswurfrate 1 024 mB/Tick | `TankMultiblockData` |
| Edit-Modi Both / Fill / Empty (Standard Both) | `ContainerEditMode` |
| Valve-Puffer 32 000 mB (Horizon-Tank, pro Tick mit Master getauscht) | Eigene Anpassung an Horizons Fluid-API |
| Zellkapazitäten 8e9, 64e9, 512e9, 4e12 | `InductionCellTier` |
| Provider-Leistung 256 000, 2 048 000, 16 384 000, 131 072 000 | `InductionProviderTier` |
| Kapazität = saturierende Summe der Zellen, Übertragung = Summe der Provider | `MatrixMultiblockData`, `MatrixEnergyContainer` |
| Mindestens 1 Zelle und 1 Provider | `MatrixValidator.postcheck` |
| Größe 3..=18 pro Achse | `MultiblockManager` / Validator-Konstanten |
| Stärke 5/9 (Casing, Valve, Port), 3,5/16 (Zellen, Provider, Glas) | `BlockBasicMultiblock`, `BlockTile` |
| Rahmen/Flächen/Innenraum-Regeln (Edge nur Frame, Face Frame/Valve/Controller, Inside Air bzw. Interior) | `FormationProtocol`, `CuboidStructureValidator` |

## Entwurf

- Eine Struktur ist die verbundene Hülle (Flood-Fill) mit Bounding Box. Ergebnisse: `Formed`, `Invalid(Grund)`, `Unloaded`. `Unloaded` löst nie etwas auf.
- Der Master hält den gemeinsamen Zustand (NBT-Schlüssel `mekanism_mb_*`). Beim Zerstören eines Masters per Hand geht der Inhalt an ein benachbartes Mitglied; bei Explosionen/Befehlen geht er verloren.
- Es gibt keine `block_changed`-/`chunk_load`-Hooks. Mitglieder prüfen per Polling in `block_entity_tick`, gestaffelt nach Positions-Hash: ungeformte alle 200 Ticks (und sofort bei `place_block`), geformte Mitglieder ihren Link alle 10 Ticks, der Master seine Struktur alle 20 Ticks.
- Induction Port: Horizon-Energiepuffer (max. `PORT_BUFFER` = 131 072 000), Anteil pro Tick = Übertragung / Portanzahl. Input leert den Puffer in die Matrix, Output füllt ihn und gibt per `give()` an Mekanism-Nachbarn ab.

## Lücken und Blocker

- **Thermal Evaporation Plant fehlt.** Validator-Regeln (offene Oberseite, Controller-Anzahl) und `Type::Evaporation`-Stubs sind vorbereitet, Host-Logik fehlt.
- **Mekanism-Quellen laden Ports nicht per `give()`.** `energy::acceptor` in `energy.rs` ist eine ID-Tabelle (gehört nicht zu dieser Session). Horizon-Kabel nutzen den Puffer. Hinweis an die Lead-Session: Ports in die Tabelle aufnehmen.
- Structural Glass ist (wie upstream) kein Teil von Tank oder Matrix.
- Inhalt (Fluid/Energie) liegt im Master. Zellen haben keine Block-Entity. Wird der Master ohne benachbartes Mitglied zerstört, ist der Inhalt weg.
- Keine Energie-/Item-Slots, kein Redstone-Ausgang, kein `shouldCache`-Verhalten für Zellenenergie.
- Edit-Modus wirkt nur über den Puffer-Austausch; `accepts_fluid` wird pro Modus nicht erzwungen.
- Polling statt Ereignisse: Bildung/Auflösung kann bis ca. 10 s dauern. Eine große unvollständige Hülle lässt jedes ungeformte Mitglied alle 200 Ticks neu fluten.
- Die Eigenschaft `formed` hat keinen Modellunterschied.
- Ein Valve-Puffer mit anderem Fluid als der Master bleibt hängen.

## Nicht kompilierverifiziert

Der `native-build`-Check ist verboten (Lead-Entscheid). Der native Code folgt vorhandenen kompilierten Mustern, wurde aber **nicht gebaut**. Risikopunkte:

- `host.level(&dim)` und `LevelRef::is_loaded` / `game_time`.
- `&*e.id` (Deref der Entity-ID).
- Borrow-Muster in `form()` mit `own: Option<(Pos, &mut Compound)>`.
- Typen der `change_mod_block_entity`-Closures.
- `Tag::Long`.
- Literale `ModEnergyBuffer` und `ModFluidTanks`.
- `ScreenDrawEvent.block`.
- `event.entity.data` als `Vec<i32>`.

## Geprüft

| Check | Ergebnis |
|---|---|
| `cargo test --manifest-path logic/energy/Cargo.toml` | 162 bestanden, 0 fehlgeschlagen |
| `python scripts/validate_content.py` | OK, 5737 JSON-Dateien |
| `python scripts/parity.py --check` | 13029 Einträge, 0 Regressionen, 873 neu erledigt |
| `cargo horizon-mod build` + `check` (`content/mekanism`) | OK, `mekanism 0.2.0`, 2 045 988 Bytes |

## Ingame-Testliste

Alles nur nach manuellem Build durch die Lead-Session.

Dynamic Tank:
1. Würfel 3×3×3 aus Dynamic Tank, innen leer: nach spätestens ca. 10 s formen sich die Blöcke; Screen zeigt "Capacity 16 000 B × Volumen" (27 → 432 000 B).
2. Eine Kante mit Dynamic Valve ersetzen: Valve wird nicht angenommen (Kante braucht Frame). Valve in eine Fläche setzen: gültig.
3. Pipe an Valve anschließen, Wasser einleiten: Füllstand im Screen steigt.
4. Edit-Modus auf Empty stellen: Valve schiebt (1 024 mB/Tick) in angrenzende Behälter.
5. Einen Block entfernen: Struktur löst sich, Inhalt bleibt im Master-Cache; neu setzen: Inhalt kehrt zurück.
6. Master von Hand abbauen: Inhalt geht an ein Nachbarmitglied.
7. Innenraum mit Block füllen: Struktur wird ungültig.
8. Größe 19 und 2: ungültig (Grenzen 3..=18).
9. Chunk entladen/laden: Struktur löst sich nicht auf.

Induction Matrix:
1. 3×3×3: Casing-Hülle, 1 Port in einer Fläche, 1 Zelle + 1 Provider innen, Rest Luft: formt sich.
2. Ohne Provider oder ohne Zelle: ungültig (`postcheck`).
3. Capacity im Screen: Basic 8 G, Advanced 64 G, Elite 512 G, Ultimate 4 T; Transfer entsprechend 256 k … 131,072 M.
4. Port im Input-Modus: Kabel/Generator füllen Matrix. Port per Button auf Output: Matrix lädt angrenzenden Verbraucher.
5. Mehrere Ports: Gesamtübertragung teilt sich auf alle Ports.
6. Mekanism-Energiequelle direkt neben dem Port: lädt **nicht** (bekannte Lücke).
7. Zelle abbauen: Kapazität sinkt, Energie wird auf Kapazität geklemmt.

Structural Glass:
1. Setzen und prüfen, dass er keine Tank-/Matrix-Struktur bildet.
2. Hülle mit Glas statt Casing: bleibt ungültig (upstream-konform).

## Offene Punkte

- Evaporation Plant (Schritt 4) implementieren.
- `energy::acceptor` um Induction Port erweitern (Lead, `energy.rs`).
- Native Kompilierung der neuen Module in einer erlaubten Umgebung prüfen.
- Optional: Modellvariante für `formed`, Ereignis-basierte Revalidierung, Redstone-Ausgang.
