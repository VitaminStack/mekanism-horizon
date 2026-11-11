# Item-Hooks und Ausrüstung (M11)

Prüfung der Horizon-Item-Hooks gegen den Pin `fdc87ae` (Native-Hooks-API 411) und die Machbarkeit jedes M11-Items. Quellen: `native-hooks.json` und `docs/native-mods.md` des Horizon-Checkouts, der generierte `Hooks`-Trait (`mod_api_generated.rs`). Umsetzung: `native/mekanism_native/api/src/items.rs` (Hooks) und `gear.rs` (Regeln und Zahlen, ohne Horizon-Typen, Tests in `logic/energy`).

> Korrektur zu [TOOLS.md](TOOLS.md): Dort steht, Horizon habe keinen `use_on`-Hook. Das stimmt für den Content-Pfad, nicht für die native Mod: `use_item_on` (API 13) hört einen Rechtsklick mit Item auf einen Block, den der Block durchgelassen hat. Die Paxel-Schaufelaktion ist damit native machbar (noch nicht gebaut).

## Vorhandene Hooks

"Server" = läuft auf dem Server-Thread (Einzelspieler, LAN, dediziert). Alle Hooks hören nur und ändern den übergebenen Event, sofern nicht anders vermerkt. Blockierende Lücken stehen unten.

| Bedarf | Hook / Host-Aufruf | API | Anmerkung |
|---|---|---|---|
| Rechtsklick auf Block mit Item | `right_click_block` (vor Block und Item, abbrechbar), `use_item_on` (nur wenn der Block durchlässt) | 21, 13 | Maschinen und Tanks öffnen ein Menü, deshalb für Items auf Tanks `right_click_block` |
| Rechtsklick in der Luft | `use_item` (abbrechbar = beansprucht) | 13 | Sneak-Status über `Host::sneaking` |
| Linksklick auf Block, Schlag auf Mob | `left_click_block`, `attack_entity`, `hurt_enemy` | 21, 39 | |
| Schaden ändern | `living_incoming_damage_v2` (Waffe als Snapshot, `amount` änderbar, Host) | 384 | nur Nahkampf/Sweep/Speer des Spielers auf Mobs; v1/v3 ohne Waffe |
| Item pro Tick im Inventar | `inventory_tick(slot, selected)`, `carried_stack(slot)` | 39 | Rüstungsslots 36–39, `carried_stack` nur innerhalb des Hooks; Haupthand = der Tick mit `selected` |
| Stack außerhalb davon ändern | `Host::held_stack(uuid)` liest; `set_inventory_slot` ist in den Use-Hooks **nicht nutzbar** (Spielerzustand getauscht, getestet) | 39+ | Änderungen werden in `items.rs` vorgemerkt und im nächsten `inventory_tick` über `carried_stack` geschrieben |
| Ausrüsten | `on_equip` (Spieler), `living_equipment_change` (nur Mobs und Rüstungsständer) | 39, 148 | Es gibt keine Host-Abfrage der Offhand (`player_stacks` kennt nur die 36 Hauptslots); das Dictionary merkt sie sich aus `on_equip`, sieht also ein Item erst nach dem nächsten Wechsel |
| Item-Zustand | `minecraft:custom_data` lesen über `ItemStack::custom_data`; schreiben nur durch Neuaufbau des Stacks mit `Host::item_from_snbt` | 65 | `Host::set_component` gilt nur für eigene, per `component_types` registrierte IDs: für `minecraft:custom_data` kommt `UnknownType` (getestet) |
| Standardkomponenten | `modify_default_components` | 79 | |
| Item-Energie | `item_energy_buffer` (Kapazität, Ein-/Ausgabe) | 307 | Horizon speichert `horizon_energy` in `custom_data` und lädt Items in Cubes/Tablet-Slots selbst |
| Item-Fluidtank | `item_fluid_tank` | – | wird hier nicht genutzt: Chemikalien sind Fluide mit `mekanism:chemical/`-Präfix und stehen in `custom_data` (`mekanism_fluid`, `mekanism_amount`, wie bei den Tank-Items) |
| Benutzen über Zeit | `use_duration`, `use_animation`, `can_eat`, `living_entity_use_item_start/finish/tick`, `on_use_tick`, `release_using`, `living_entity_use_item_stop` | 90, 131 | Item ohne Vanilla-Verwendung mit `use_duration > 0` wird gehalten (Trinken, Laden) |
| Essen, Hunger | `Host::player_food`, `set_player_status`, `food_level_changed` | 169 | |
| Bogen | `arrow_loose` (abbrechbar), `entity_shoot_bow`, `living_get_projectile` | – | |
| Spielerbewegung | `Host::entity_velocity/entity_set_velocity`, `push_player`, `set_flight`, `set_flying_speed`, `player_tick`, `player_toggle_flight`, `player_velocity`, `entity_toggle_glide` | 31, 153 | Server setzt Geschwindigkeit; kein Client-Vorhersage-Hook dafür |
| Atmen | `living_breathe(can_breathe, consume, refill)` | 144 | Spieler und Mobs mit generischer Luftlogik |
| Tasten | `input_key`, `key`, `mod_key` (Client), `net_send_to_server`/`net_from_client` | 40 | Eigene Taste braucht Client-Hook plus Nachricht an den Server |
| Item-Anzeige | `item_bar` (Balken), `item_decorations` (Zeichnen), `tooltip`, `tooltip_component(_draw)` | 60, 89, 12, 68 | |
| Eigenes Item-Modell je Zustand | `item_model_property` | 385 | Z. B. Geiger-Counter-Stufen, Jetpack-Modus |
| Items/Blöcke registrieren | Content-Mod (`data/<ns>/horizon/item/*.json`: Vanilla-`template` plus `components`) oder `items`-Hook | – | Nur Vanilla-Verhalten per Komponenten: Werkzeug, Rüstung (`equippable`), Elytra (`glider`), Essen |
| Block abbauen | `destroy_block` (abbrechbar), `Host::destroy_block(pos, Werkzeug, collect)` | 14 | Vein-Mining ruft das für Nachbarn auf |
| Chat/Hinweis | `Host::tell`, `action_bar`, `feedback` | – | |

Nicht vorhanden oder nicht nutzbar:

- Kein Hook, der die Tags eines Blocks oder Items **auflistet**: nur Mitgliedschaftstest je Tag (`block_name_in_tag`, `server_item_in_tag`). Das Dictionary prüft deshalb eine generierte Liste (`scripts/gen_dictionary_tags.py`, alle Tags der Content-Mods plus ein paar Vanilla-Tags).
- Keine dynamischen Attribute je Stack-Zustand: Angriffswert, Abbaugeschwindigkeit und Rüstungswerte kommen aus der statischen Komponente. Das Disassembler-Waffenplus läuft deshalb über `living_incoming_damage_v2`.
- Kein Strahlungs-, Radiation- oder Modulsystem im Port; Item-NBT allein trägt das nicht.
- Kein Hook für Spieler-Fallschaden außer `living_incoming_damage` (v1: Schadensart, änderbare Menge, ohne Host) – für Free Runners ausreichend, aber ohne Rüstungsstack.
- Kein Client-Vorhersage-Hook für Bewegung: Jetpack-Schub setzt serverseitig die Geschwindigkeit, im LAN und auf dedizierten Servern kann das ruckeln (ungetestet).
- GUI-Klicks mit einem Item auf der Maus (Gauge Dropper auf einem GUI-Gauge) sind nur über `menu_clicked`/`screen_click` des jeweiligen Menüs erreichbar; die Mekanism-Gauges gibt es nur in den eigenen Tank-Screens.

## Machbarkeit je Item

Status: **umgesetzt** (in dieser Branch, ingame getestet, siehe [TESTLOG.md](TESTLOG.md)), **machbar** (Hooks vorhanden, nicht gebaut), **blockiert** (Zutat oder System fehlt im Port).

| Item | Status | Hooks / Grund |
|---|---|---|
| Atomic Disassembler | umgesetzt (teilweise) | Energiepuffer 1.000.000 FE, Abbau Tempo 20 für Spitzhacke/Axt/Schaufel/Hacke (Komponente), ohne Haltbarkeit (Stock-Template; `minecraft:unbreakable` ist in Horizon-Content kein bekannter Komponententyp), Waffenplus 16 für 2.000 FE (`living_incoming_damage_v2`), Modus per Sneak+Rechtsklick (`use_item`), Vein-Modus (`destroy_block` plus `Host::destroy_block`, Tag `mekanism:atomic_disassembler_ore` = `#c:ores` + Stämme, max. 128 Blöcke, 10 FE je Block), ohne Energie wird kein Block abgebaut. Fehlt: Modi Slow/Fast/Off (Abbautempo ist statisch; ob `set_component` die `minecraft:tool`-Komponente eines Stacks ändern darf, ist ungeprüft), Energie aus dem Tablet-Slot, Konfigurationswerte |
| Electric Bow | machbar | `item_energy_buffer`, `arrow_loose` (Pfeil verbrauchen/Energie abbuchen, Brandmodus), Vanilla-`bow`-Template; Rezept braucht nur Zutaten, die es gibt |
| Flamethrower | machbar | Gas im Item (Wasserstoff, `custom_data`), `use_duration` plus `on_use_tick` für Dauerfeuer, Feuer über `Host::set_block`/Partikel; Zutaten (Chemical Tank, `#c:tools/igniter`) vorhanden. Modi und Mob-Brand ungeprüft |
| HDPE-Reinforced Elytra | blockiert (Zutat) | Content: `minecraft:glider`/`equippable`-Komponenten; Rezept braucht `mekanism:hdpe_sheet` (M06) |
| Jetpack, Armored Jetpack | machbar | Brustrüstung (Template plus `equippable`), Hydrogen im Item, Schub per `entity_set_velocity` aus `inventory_tick`/`player_tick`, Modus und Taste per `input_key` plus `net_send_to_server`; Rüstungsmodell/-textur liegen als Upstream-Assets vor. Rüstungswerte nicht aus Java geprüft |
| Free Runners, Armored Free Runners | machbar | Energiepuffer, Fallschaden über `living_incoming_damage` (Menge senken), Stiefel-Template; Rüstungswerte unverifiziert |
| Scuba Mask und Tank | machbar | `living_breathe` (`can_breathe`, `consume`) mit Sauerstoff aus dem Tank-Item im Rüstungsslot; Zutaten vorhanden. Wasseratmung und Nachtsicht-Effekt (`add_effect`) möglich |
| Hazmat Suit | blockiert (System) | Rüstung selbst wäre Content, Strahlungsschutz braucht ein Radiation-System, das der Port nicht hat; Rüstungswerte nicht verifiziert |
| MekaSuit (alle Teile), Meka-Tool, Modification Station | blockiert (System) | Modulsystem (Modulitems, Slots, Konfig-Screens, Energieverbrauch je Modul) existiert nicht; Zutaten (Pellets, Teleportation Core u. a.) fehlen größtenteils |
| Canteen | umgesetzt | Fassungsvermögen 64.000 mB Nutritional Paste in `custom_data`, 50 mB je Hungerpunkt, 32 Ticks Trinken (`use_duration`, `use_animation` Drink, `living_entity_use_item_start/finish`), Item-Balken und Tooltip. Auffüllen im Original per Nutritional Liquifier (fehlt): hier Rechtsklick auf einen Tank mit Paste (1.000 mB je Klick) |
| Gauge Dropper | umgesetzt (teilweise) | 16.000 mB Fassungsvermögen, Rechtsklick auf Tank/Maschine mit Fluidtank zieht 16 mB eines Fluids oder einer Chemikalie, Sneak plus Rechtsklick gibt 16 mB zurück, Sneak plus Rechtsklick in die Luft leert. Original arbeitet auf GUI-Gauges; das bleibt offen |
| Geiger Counter, Dosimeter | blockiert (System) | kein Radiation-System; Item-Modellzustände (`item_model_property`) wären machbar |
| Dictionary | umgesetzt | Rechtsklick auf Block: Block- und Item-Tags; Rechtsklick in der Luft: Tags des Items in der Offhand. Nur Tags aus der generierten Liste (kein Listen-Hook) |
| Crafting Formula | blockiert (System) | gehört zum Formulaic Assemblicator (fehlt); Item-NBT für Rezeptlisten wäre machbar |
| Network Reader | blockiert (System) | liest Transmitter-Netze (Kabel, Rohre; M07/M08 offen) |
| Portable QIO Dashboard | blockiert (System, Zutat) | braucht QIO-Dashboard und Frequenzen (M10 offen), Rezept braucht Polonium-Pellets |

## Nicht gebaute Items

Items ohne Funktion werden bewusst nicht registriert (Geiger Counter, Dosimeter, Crafting Formula, Network Reader, Portable QIO Dashboard, Hazmat, MekaSuit, Meka-Tool, HDPE-Elytra, Jetpack, Free Runners, Scuba, Electric Bow, Flamethrower). Ein craftbares Item, das nichts tut, wäre eine Vortäuschung. Reihenfolge der nächsten Kandidaten: Electric Bow, Free Runners, Scuba, Jetpack, Flamethrower.

## Werte

Die Mekanism-Zahlen (Canteen-Kapazität und -Rate, Dropper-Schrittweite, Disassembler-Energie und -Kosten, Vein-Limit) sind Mekanisms Standardwerte nach Quellenkenntnis, **nicht** aus den Java-Quellen gelesen: der Upstream-Checkout für die Assets (`31bb4906`) ist ein Sparse-Checkout ohne `src/main/java`. Sie stehen gesammelt in `gear.rs` und sind bei einem vollständigen Upstream-Checkout abzugleichen.
