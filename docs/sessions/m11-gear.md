# M11 Gear: Jetpack, Free Runners, Scuba, Elektrobogen, Flammenwerfer

Branch `claude/m11-gear-5ad777`. Stand: Logik per Unit-Test geprüft, Content validiert und als `.hmod` gebaut,
**Native-Code nicht kompiliert** (der `native-build`-Check ist für diese Session gesperrt). Alles in `worn.rs`,
den Erweiterungen in `items.rs`/`lib.rs` und `gear.rs` ist ungetestet im Spiel.

## Was gebaut wurde

| Item | Werte (Quelle: Mekanism 26.3 Java unter `src/main/java/mekanism/common/`) |
|---|---|
| Jetpack / Armored Jetpack | 24000 mB Wasserstoff (`item/gear/ItemJetpack`, `config/GearConfig`), 1 mB/Tick, Schub 0,15 (`item/interfaces/IJetpackItem`), Modi Regular/Hover/Vector/Disabled, Armored: Rüstung 8, Zähigkeit 2 (`MekanismStartupConfig`) |
| Free Runners / Armored | 64000 FE, Laderate 320, 50 FE je Schadenspunkt, Verhältnis 1,0, Sicherheitsfallhöhe 3 (`ItemFreeRunners`, `IFreeRunnerItem`, `CommonPlayerTickHandler.livingFall`), Modi Normal/Safety/Disabled, Step +0,5 nur Normal und nicht schleichend. Armored: Rüstung 3, Zähigkeit 2 |
| Scuba Mask / Scuba Tank | Tank 24000 mB Sauerstoff, Maske Kopf, Tank Brust (`ItemScubaTank`, `ItemScubaMask`); je Tick mindestens 1 mB, sonst so viel wie die Luft auffüllt |
| Electric Bow | 120000 FE, Laderate 600, 120 FE je Schuss, 1200 mit Feuermodus (`ItemElectricBow`), Verzauberbarkeit 1 |
| Flamethrower | 24000 mB Wasserstoff, 1 mB/Tick, Modi Combat/Heat/Inferno, Schaden 10, 5 s Brand, Reichweite (`ItemFlamethrower`, `entity/EntityFlame`) |

Reine Regeln mit Unit-Tests in `native/mekanism_native/api/src/gear.rs` (`cargo test --manifest-path logic/energy/Cargo.toml`:
139 Tests grün): Fallformel (`absorb_fall`), Schrittweite, Bogenkosten, Scuba-Verbrauch, Jetpack-Bewegung/-Aktivierung,
Modi, Kegel des Flammenwerfers. Hook-Anbindung in `worn.rs`; `items.rs` hat Fass-Befüllung, Balken, Tooltips;
`lib.rs` nur additive Weiterleitungen.

Content (nur über Generatoren): `scripts/gen_resources.py` (Block „worn and held powered gear“), danach
`import_upstream.py`, `import_generator_models.py`, `gen_dictionary_tags.py`. Rezepte, Modelle, Texturen, Namen
kamen vom Import. Der Lauf hat auch bereits veraltete generierte Dateien nachgezogen (z. B. Transmitter-Tags,
Reihenfolge in `mineable/pickaxe.json`).

## Bedienung

- **Befüllen**: Rechtsklick mit Jetpack/Scuba-Tank/Flammenwerfer auf einen Chemical Tank mit Wasserstoff bzw. Sauerstoff, 1000 mB je Klick (Mekanism füllt über Slot/Condensentrator).
- **Modus**: Schleichen + Rechtsklick mit dem Item in der Hand (Runners, Jetpack, Flammenwerfer, Scuba-Tank an/aus, Bogen Feuermodus).
- **Jetpack**: Leertaste = steigen, Shift = sinken (Hover). Läuft auf dem Client.

## Abweichungen und Lücken (nichts vorgetäuscht)

1. **HDPE-Reinforced Elytra**: nicht gebaut, `mekanism:hdpe_sheet` ist nicht registriert (0 Treffer in `scripts/registry.json`).
2. **MekaSuit, Meka-Tool, Hazmat, Geiger, Dosimeter**: wie beauftragt nicht gebaut.
3. **Jetpack-Schub auf dem Client**: `entity_set_velocity`/`entity_push` sind für Spieler wirkungslos (der Client bewegt sich selbst). Darum liest `client_tick_post` Leertaste/Shift, schätzt die Geschwindigkeit aus den Positionen der letzten zwei Ticks (`on_ground` ≈ „keine y-Änderung“) und ruft `push_player`; der Server bucht den Treibstoff nach einer Nachricht (`mekanism:jet_thrust`, 100 ms Takt, 300 ms gültig) und meldet Modus/Treibstoff zurück (`mekanism:jet_state`). Hover/Vector sind eine Näherung an `IJetpackItem.handleJetpackMotion`. Auf LAN/Servern kann es ruckeln. Der Elytra-Gleitzweig fehlt. Das Zurücksetzen der Fallstrecke ist genähert: Fallschaden entfällt, wenn der Server in den letzten 0,5 s Schub gesehen hat.
4. **Keine Tastenbelegung zum Moduswechsel getragener Rüstung**: Nur über Schleichen + Rechtsklick mit dem Teil in der Hand (Mekanisms eigener Mode-Key braucht `mod_key`/`key`, ungetestet und nicht gebaut). Scuba-Tank ist hier **standardmäßig an** (Mekanism: aus), sonst wäre er ohne Taste nicht benutzbar.
5. **Rüstungsmodell am Körper fehlt**: Mekanisms Jetpack/Scuba/Runners-Modelle sind Java-Renderer, es gibt kein `equipment`-JSON. Items sind `minecraft:stick` + `minecraft:equippable` ohne `asset_id`, ohne Haltbarkeit.
6. **`living_fall`, `living_breathe` haben keinen Host**. `inventory_tick` legt je Entity-ID einen Schnappschuss (max. 1 s alt) von Stiefeln/Scuba ab; die Energie für einen Sturz wird beim nächsten Tick abgebucht (nicht im Hook). Schnappschuss wird überschrieben, nie addiert, weil der Hook auf beiden Seiten läuft. Zwei Stürze im selben Tick zahlen nur einmal.
7. **Armour-Slots**: Der Hook-Doc sagt 100 bis 103, `docs/ITEM_HOOKS.md` sagt 36 bis 39. `worn.rs` akzeptiert beide.
8. **Step-Assist**: über `set_attribute_base` auf `minecraft:step_height` (0,6 auf 1,1), beim Ablegen zurückgesetzt. Ungetestet; Basiswert 0,6 ist angenommen, andere Mods, die ihn ändern, werden überschrieben. Mekanisms zusätzliches Bewegungseffizienz-Attribut fehlt.
9. **Scuba**: Mekanisms Schutz vor „magischem“ Schaden und die Beschleunigung bei vollem Luftvorrat fehlen. Der Sauerstoffverbrauch von 1 mB/Tick an Land entspricht der Java-Logik. Wasserspezifischer Sicht-/Aqua-Affinity-Effekt der Maske nicht geprüft.
10. **Bogen**: Schuss wird in `arrow_loose` ohne Energie abgebrochen; die Energie wird erst beim Tick nach `entity_shoot_bow` abgebucht. Feuermodus: NBT `Fire` auf dem Pfeil per `merge_entity_nbt`; ob das den Pfeil wirklich brennen lässt, ist ungetestet (Mekanisms Weg ist die Flame-Verzauberung).
11. **Flammenwerfer ohne Flammen-Entity**: Horizon kann keinen eigenen Entitätstyp mit Flugbahn hinzufügen. Stattdessen Hitscan-Kegel (16 Blöcke, ±0,25 rad) alle 2 Ticks um die Augen des Spielers: Schaden 10 (`minecraft:on_fire`, der Typ `mekanism:flamethrower` existiert nicht) plus 5 s Brand über NBT `Fire`. Inferno setzt Feuer auf den ersten freien Block vor einer Fläche (bis 8 Blöcke). **Heat (Schmelzen von Blöcken/Items) fehlt**, Heat wirkt wie Combat. Keine Flammenpartikel entlang der Bahn, nur einer am Lauf. Gegenstände am Boden werden nicht zerstört.
12. **Zusätzlich (nicht mein Bereich)**: Gauge-Dropper-Rate (upstream 250 statt 16) und Vein-Mining-Standard (upstream aus) weichen von `gear.rs` ab. Follow-up für `items.rs`/`gear.rs` des Droppers.
13. **Nicht kompiliert**: `worn.rs`, `items.rs`, `lib.rs`. Signaturen sind gegen den gepinnten Horizon-Quelltext (`fdc87ae`) und die generierte `mod_api_generated*.rs` gelesen, aber nie vom Compiler gesehen. Namen, die ich nur angenommen habe: `ClientTickPostEvent` und `KeyCode::{Space,ShiftLeft}` im `mod_api`-Re-Export, `NativeServerPlayer.{yaw,pitch,entity_id,position}`.

## Ingame-Testliste

1. Ein Chemical Tank mit Wasserstoff: Rechtsklick mit Jetpack → Balken füllt sich um 1000 mB je Klick. Gleich für Scuba-Tank (Sauerstoff) und Flammenwerfer.
2. Jetpack anlegen (Rechtsklick), Leertaste halten: steigt, Treibstoff sinkt 1 mB/Tick. Kein Fallschaden nach Schub. Ohne Treibstoff: kein Schub.
3. Modi (Schleichen + Rechtsklick): Hover hält die Höhe, Shift sinkt langsam; Vector schiebt in Blickrichtung; Disabled schiebt nicht.
4. Multiplayer/LAN: Schub ruckelt? Verbraucht der Server den Treibstoff?
5. Free Runners laden (Energie-Tablet/Cube), von 10 Blöcken fallen: kein Schaden, 350 FE weg. Ohne Energie: normaler Schaden. Safety: kein Step-Boost. Normal: Stufen von 1 Block ohne Springen, nicht beim Schleichen. Stiefel ablegen: Step wieder 0,6.
6. Scuba-Maske + Tank (Sauerstoff) im Wasser: Luftanzeige bleibt voll, Tank sinkt. Tank aus (Schleichen + Rechtsklick): Luft sinkt wieder. Maske abnehmen: Luft sinkt innerhalb ~1 s wieder.
7. Elektrobogen laden, schießen: 120 FE je Schuss; ohne Energie kein Schuss. Feuermodus: 1200 FE, brennt der Pfeil?
8. Flammenwerfer gedrückt halten: Wasserstoff sinkt 1 mB/Tick, Mobs im Kegel nehmen Schaden und brennen. Inferno: Feuer auf Block. Ohne Wasserstoff startet er nicht.
9. Verzauberungen auf Bogen und Maske bleiben nach dem Laden/Verbrauchen erhalten (`edit_data` ändert jetzt direkt statt den Stack neu zu bauen).
10. Rezepte aller acht Items im Craftingtisch; Namen und Texturen.
