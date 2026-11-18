# Session m07-machine-framework (Maschinen-Rahmen: Upgrades, Energy-Slot, Redstone, Configurator)

Branch `claude/m07-machine-framework-b79164`. Nicht im Spiel getestet, Native-Code nicht kompiliert (siehe unten).

## Was portiert wurde

| Teil | Wert | Upstream-Quelle |
|---|---|---|
| Upgrade-Items `mekanism:upgrade_speed/energy/muffling` + Item-only `chemical`, `anchor`, `filter`, `stone_generator`, `xp_containment` | je ein Item pro Art (Upstream 26.3: ein Item `mekanism:upgrade` mit Datenkomponente `upgrade_type`) | `MekanismItems.UPGRADE`, `UpgradeUtils` |
| Maximalzahlen | Speed 8, Energy 8, Muffling **1** (26.3: `Upgrade.create` ohne Max = 1), Chemical 8 | `MekanismDatapackRegistries` (Upgrades) |
| Ticks | `max(1, trunc(def * 10^(-speed/8)))` | `UpgradeUtils.getTicks` |
| Energieverbrauch | `ceil(def * 10^(2*speed/8 - energy/8))` | `UpgradeUtils.getEnergyPerTick` |
| Kapazitaet | `def * 10^(energy/8)` | `UpgradeUtils.getMaxEnergy` |
| Multiplikator 10 | `maxUpgradeMultiplier` Default | `GeneralConfig` |
| Muffling | Lautstaerke `1 - muffling/max` (1 Upgrade = stumm) | `TileEntityMekanism.getSoundVolume` |
| Redstone-Modi Disabled/High/Low/Pulse | Pulse = bestromt und Vortick unbestromt | `IRedstoneControl.RedstoneControl`, `TileEntityMekanism.isRedstoneActivated` |
| Energy-Slot | laedt aus Energy Tablet, Energiewuerfel, Disassembler mit deren Laderate | `EnergyInventorySlot` |
| Rezepte der Upgrades und des Configurators | importiert (Ergebnis auf Einzel-Items umgeschrieben) | `UpgradeRecipeProvider`, `configurator.json` |
| Configurator (Modus Rotate) | Rechtsklick dreht die Front zur geklickten Seite (Maschinen nur horizontal, Wuerfel alle sechs Seiten) | `ItemConfigurator.useOn` (ROTATE) |

Code: `native/mekanism_native/api/src/upgrade.rs` (Horizon-frei, Tests in `logic/energy`), `machine.rs` (Tabelle: `all_slots`, `energy_slot`, `upgrade_slot`, `upgrade_out_slot`, `upgraded`, `data_slots`), `process.rs` (Tick, Bildschirm, Buttons), `lib.rs` (Hooks `use_item_on`, `menu_slot_rules`, Lautstaerke, `item_buffer`).
Generatoren: `scripts/gen_resources.py` und `scripts/import_upstream.py`, je ein markierter M07-Block.

## Bedienung

* Slots nach den eigenen Slots der Maschine, angehaengt (alte Speicherstaende bleiben gueltig): Energy-Slot, Upgrade-Eingang, Upgrade-Ausgang (Results-Slot).
  Positionen (Unterkante, `machine::ENERGY_SLOT_POS` usw.): x 98 / 118 / 138, y 54.
* Upgrade in den Eingang legen: wird sofort installiert (soweit Platz). Die kleinen Knoepfe `S`, `E`, `M` (zeigen die installierte Zahl) nehmen per Klick eines der Art in den Ausgang zurueck.
* Knopf `RS:` schaltet Disabled, High, Low, Pulse durch.
* Solar Neutron Activator: keine Slots/Upgrades/Redstone (Upstream: nur XP-Containment).

## Luecken und Blocker

* **Smelter**: `minecraft:smelting` bleibt nicht erreichbar. `recipes_of_type` liefert nur Nicht-`minecraft`-Typen, `minecraft:*`-Rezepte sind intern (`crafting.rs`, Aufteilung `cooking`/`custom`). Energized Smelter bleibt ungeportet.
* **Energiekapazitaet** ist in Horizon pro Blocktyp statisch (`native_block_entity_energy`, einmal beim Aufbau gefragt). Deklariert ist das Maximum (Energy-Upgrades x8 = 10-fach); der Tick begrenzt den gespeicherten Wert auf die tatsaechliche, hochgerechnete Kapazitaet. Fremdquellen koennen dadurch zwischen zwei Ticks bis zum 10-fachen einspeisen, der Rest geht verloren.
* **Seitenkonfiguration/Auto-Eject**: Horizon hat statische Seiten pro Blocktyp (Hopper-Faces sind Teil von `ModContainer`, nicht pro Block-Entity). Eine pro Seite umschaltbare Konfiguration der Maschinen und der Configurator-Modi Items/Fluids/Chemie/Energie/Waerme ist nicht portierbar, Auto-Eject der Maschinen bleibt wie in M06 (Tanks) bzw. aus. Energiewuerfel haben ihre eigene Seitenkonfiguration (M03).
* Configurator: Modi Wrench und Empty fehlen (kein Item-Modus: Items tragen hier keine typisierte Komponente), Sneak-Rotation (Gegenseite) fehlt, keine Chatmeldung. Die Modell-Varianten `configurator_empty/rotate/wrench` sind importiert, aber ungenutzt.
* Chemical-/Anker-/Filter-/Stone-Generator-/XP-Upgrades: nur Items, Maschinen nehmen sie nicht an (Effekt fehlt: Gasverbrauch, Chunkloading, Pumpe, Cobblestone Generator, XP).
* Upgrades bleiben nicht am abgebauten Block (Upstream: Datenkomponente am Item); `loot_drops` kopiert sie nicht. Beim Abbau gehen sie verloren (Folgearbeit: Drop als Items oder SNBT-Komponente wie bei Energiewuerfeln).
* Shift-Klick (`menu_quick_move`) in die neuen Slots nicht implementiert; Spieler-Klicks sind durch `menu_slot_rules` geregelt, Trichter erreichen die neuen Slots nicht (nicht in `ModContainer.faces`).
* Pulse: ein Tick Freigabe pro Flanke (wie Upstream); bei langen Operationen heisst das, dass nur ein Tick Fortschritt entsteht.
* Fortschrittsanzeige rechnet in Tabellen-Ticks um, damit die Leiste bei Speed-Upgrades stimmt. Die Reaktionskammer behaelt ihre Rezeptdauer (Speed wirkt nur ueber Energieverbrauch).
* Layout der drei Slots/Knoepfe ist frei gewaehlt (ein 176x166-Bildschirm statt Upstreams Upgrade-Fenster); Kollisionen mit Tank-Anzeigen einiger Maschinen sind nicht geprueft.

## Nicht kompilierbar geprueft

Der Native-Build lief nicht (Vorgabe der Koordination). Nicht gegen den Compiler geprueft, nur gegen die Horizon-Quelle gelesen:
`UseItemOnEvent` (`stack`, `pos`, `face`), `MenuSlotRulesEvent` (`menu`, `slot`, `stack`, `may_place`), `host.neighbor_signal(&dimension, pos)`, `host.item_from_snbt`, die Sichtbarkeit `items::edit_data` (jetzt `pub(crate)`). `rustfmt` parst alle geaenderten Dateien.

## Checks

* `cargo test --manifest-path logic/energy/Cargo.toml`: 144 Tests ok (neu: `upgrade::*`, `machine::extra_slots_*`, `upgrades_change_*`, `menu_files_match_the_table`).
* `python scripts/validate_content.py`: ok. `python scripts/parity.py --check`: 0 Regressionen.
* `cargo horizon-mod build` / `check` fuer `content/mekanism`: ok.

## Ingame-Checkliste (gebuendelt)

1. Spiel mit `Build-Content.ps1` bauen. Beim Start: keine Ladefehler; `upgrade_*` und `configurator` im Kreativ-Inventar mit Modell und Namen.
2. Rezepte: Speed-Upgrade (Osmiumstaub, Infused Alloy, Glas), Muffling (Wolle + Muffling-Zentrum), Configurator.
3. Enrichment Chamber oeffnen: Energy-Slot, Upgrade-Eingang, Upgrade-Ausgang sichtbar, nichts ueberlappt, Knoepfe `RS:Disabled`, `S0 E0 M0` sichtbar. Alte Maschine eines frueheren Speicherstands laedt ohne Verlust von Eingabe/Ausgabe.
4. Energy Tablet (voll) in den Energy-Slot: Maschine laedt mit 5000 FE/t bis zur Kapazitaet, Tablet entlaedt sich.
5. Speed-Upgrade x1 einlegen: verschwindet, Anzeige `S1`, Verbrauchs-Tooltip steigt, Operation dauert ~149 statt 200 Ticks. x8: 20 Ticks, 800 FE/t (Crusher: 8*100).
6. Energy-Upgrade: Kapazitaet und Verbrauch laut Tooltip. Klick auf `S`/`E`: ein Upgrade im Ausgang, Zaehler sinkt; Ausgang nimmt keine Items vom Spieler.
7. Muffling x1: Betriebsgeraeusch verstummt; nach Entfernen wieder da.
8. Redstone: High = nur mit Signal, Low = nur ohne, Pulse = eine Operation-Tick pro Flanke; Modus bleibt nach Neustart.
9. Configurator auf Maschine/Energiewuerfel rechtsklicken: Front dreht zur geklickten Seite; Oberseite einer Maschine wird ignoriert.
10. Solar Neutron Activator oeffnen: wie vorher, ohne Extras.
11. Energy-Slot nimmt nur Energie-Items, Upgrade-Eingang nur Upgrades (auch per Shift-Klick pruefen).
