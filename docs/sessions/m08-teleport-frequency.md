# M08: Frequenzen, Teleporter, Portable Teleporter, Quantum Entangloporter

Branch `claude/m08-teleport-frequency-b0a9ab`. Alle Werte stammen aus Mekanism 26.3 (Java), 1:1 übernommen, sofern nicht unter „Lücken“ genannt.

## Umfang

| Modul (`native/mekanism_native/api/src/`) | Inhalt | Horizon-frei / Tests |
|---|---|---|
| `frequency.rs` | Frequenz-Register (Name + Besitzer + öffentlich/privat), geteilte Puffer, `exchange()`, Panel-Buttons, Speichern/Laden | ja, 14 Tests |
| `teleporter.rs` | Kosten, Rahmenerkennung, Zielposition, Status, Gate (Cooldown) | ja, Tests in `logic/energy` |
| `teleporters.rs` | Host: Teleporter-Block, Frequenz-Panel, tragbarer Teleporter | nein |
| `entangloporter.rs` | Host: Quantum Entangloporter | nein |

`lib.rs`: nur minimale Hooks (Kind-Varianten `Teleporter`/`Entangloporter`, Match-Arme, frühe Zweige in `item_energy_buffer`, `use_item`, `tooltip`, `inventory_tick`, `item_bar`, neuer Hook `player_tick`). `items.rs` unverändert.

Content: `mekanism:teleporter`, `teleporter_frame`, `quantum_entangloporter`, `portable_teleporter`, `teleportation_core` (Generator `gen_resources.py`, Block „M08 frequencies“). Modelle, Texturen, Lang, Loot und Rezepte kommen über Generator + Importer.

## Portierte Werte (Quelle Java)

- Teleporter (`TileEntityTeleporter`, `UsageConfig`, `StorageConfig`): Grundkosten 100 FE, 2 FE/Block Distanz, Dimensionswechsel +1000 FE, Verzögerung 5 Ticks, Energiespeicher 1000 FE.
- Koordinatenskala: Nether 8, sonst 1. Mitfahrer-Faktor (1 + Mitfahrer). Saturierende Arithmetik.
- Statusreihenfolge: NoFrequency, NoFrame, NoDestination, NotEnoughEnergy, Ready.
- Rahmen: je Richtung erst nicht rotiert, dann rotiert. Zielposition: darüber ohne Rahmen, `below(2)` bei Rahmen DOWN, sonst `relative(dir)`. Yaw: N 180, S 0, W 90, E 270.
- Entangloporter-Puffer (Ultimate-Stufe, `InventoryFrequency`): Energie 256 000 000, Fluid 256 000, Chemikalie 8 192 000, ein Item-Slot.
- Portabler Teleporter (`GearConfig`): Energiepuffer 1 000 000, Übertragung 5 000.

## Entwurfsentscheidungen

- Zustand in modulweiten `static Mutex`, keine neuen Struct-Felder.
- Register-Datei `<world_dir>/mekanism_horizon/frequencies.txt` (tabgetrennt). Sofort bei Anlegen/Löschen, sonst höchstens alle 5 s bei geänderten Puffern. Lazy pro `world_dir` geladen.
- Aktive Teleporter werden nicht gespeichert; jeder tickende Teleporter meldet sich beim Register (ACTIVE_MS = 3000, Wanduhr).
- Teleportiert werden nur Spieler (`player_tick`, API 31). Horizon hat keinen Entity-in-AABB-Helfer.
- Frequenz-GUI: 16 nummerierte Kanäle (öffentlich/privat), Buttons Zurück, Weiter, Öffentlich-Schalter, Setzen (anlegen falls fehlt + binden), Löschen (Besitzer oder Op).
- Entangloporter „Fenster“: jedes Mitglied holt beim Tick seinen lokalen Inhalt in den Pool und füllt ein Fenster nach (Energie 256 000, Fluid 64 000, Chemikalie 512 000, Item = Stapellimit). Erhaltung ist getestet. Grund: Horizon-Energie ist `i32`, der Pool passt nicht.
- Mitglieder: Tank 0 = Fluid, Tank 1 = Chemikalie in der `fluids`-Liste. Stapel mit Komponenten bleiben im Slot. `broken()` gibt das Fenster an den Pool zurück.
- Portabler Teleporter: Benutzen in der Luft teleportiert zum nächsten aktiven Teleporter der Frequenz. Sneak+Benutzen schaltet Kanal/Modus. Energie wird über eine PENDING-Queue in `inventory_tick` abgezogen (SNBT-Umbau von `custom_data`, aus `items.rs` dupliziert, da dort privat).

## Lücken gegenüber Upstream

- Keine Texteingabe für Frequenznamen (nur 16 Kanäle).
- Kein Security Desk, keine Trusted-Liste, keine Farben.
- Nur Spieler, keine Mobs/Items.
- Teleporter-Kapazität 1000 FE (Upstream-TODO „26.3: Update these config values“): Dimensionswechsel und Distanzen über ca. 450 Blöcke sind damit unmöglich. So portiert.
- Entangloporter: Energie nur gefenstert, keine Hitze, keine Seitenkonfiguration, kein Energie-Slot.
- Portabler Teleporter ohne GUI (`open_menu` nur LAN).
- Kein Chunk-Loading.
- Register-Flush bis 5 s Verzögerung; kein `world_saved`/`server_stopping` (Signaturen nicht verifiziert). Absturz verliert bis ~5 s Pufferänderungen.
- Rezept `mekanism:mek_data` ist vom Importer zu `crafting_shaped` vereinfacht; Energieübertragung vom Tablet beim portablen Teleporter entfällt.
- Teleporter-Lichtstufe 3 nicht portiert.
- Kein Rezept-Zutat fehlt (alle Zutaten laut Content-Check auflösbar); `teleportation_core` hat ein Rezept.

## Nicht kompilierverifiziert

Die native Crate wurde nicht gebaut (Vorgabe des Leads: kein `native-build`). Aus Quelltext/Generatorregeln abgeleitet, nie kompiliert:
`player_tick`-Signatur und `PlayerTickEvent`-Import, `host.world_dir()`, `host.teleport_player`, `event.tank` als `usize`, `host.max_stack_size`, die neuen Match-Arme in `lib.rs`, sämtlicher Code in `teleporters.rs` und `entangloporter.rs`.
Geprüft wurden stattdessen: 159 Unit-Tests (`cargo test --manifest-path logic/energy/Cargo.toml`), `scripts/validate_content.py`, `scripts/parity.py --check`, `cargo horizon-mod build` + `check` für das Content-Paket.

## Ingame-Testliste

1. Spiel startet, Content lädt, fünf neue Items/Blöcke im Kreativ-Inventar.
2. Teleporter + Rahmen setzen, GUI öffnet, Kanal wählen, Setzen, Status wechselt (NoFrequency → NoFrame/NoDestination).
3. Zwei Teleporter mit gleicher Frequenz, je mit Energie versorgt, Rahmen darum: Spieler im Feld wird nach 5 Ticks teleportiert, Energie sinkt um Kosten.
4. Zu wenig Energie → Status NotEnoughEnergy, keine Teleportation. Dimensionswechsel erwartet scheitern (1000 FE).
5. Privat/öffentlich: zweiter Spieler sieht private Kanäle des ersten nicht. Löschen nur als Besitzer/Op.
6. Welt neu laden: Frequenzen vorhanden, Entangloporter-Inhalt nach Neustart (≤5 s Verlust).
7. Zwei Entangloporter gleicher Frequenz: Energie/Fluid/Chemikalie/Item verteilt sich, Abbau gibt Inhalt zurück, nichts dupliziert.
8. Portabler Teleporter: Sneak+Benutzen wechselt Kanal, Benutzen teleportiert zum nächsten aktiven Teleporter, Energie sinkt.
