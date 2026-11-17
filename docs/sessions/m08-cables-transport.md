# Session m08-cables-transport: Universal Cables und Logistical Transporter

Branch `claude/m08-cables-transport-d803b6`. Nichts davon wurde im Spiel getestet
(Build-Content, Build-Native und der Spielstart waren fuer diese Session gesperrt).

## Was portiert ist

### Task A: Universal Cables (TODO M03)

Die vier Bloecke `mekanism:{basic,advanced,elite,ultimate}_universal_cable`.

- Werte aus `CableTier.java`: Kapazitaet 8.000 / 128.000 / 1.024.000 / 8.192.000. Die Rate je Tick ist gleich der Kapazitaet.
- Jedes Kabel ist ein Block Entity mit eigenem Horizon-Energiepuffer (`block_entity_energy`, NBT `energy`).
  Es nimmt auf jeder Seite beliebig viel bis zur Kapazitaet an und gibt ueber die Capability nichts ab.
- Im Tick verteilt das Kabel selbst (`transmitter::distribute`): zuerst an Endpunkte (Cubes, Maschinen, Pumpen),
  danach Ausgleich (Fuellstand angleichen) mit Nachbarkabeln. Gegeben wird ueber `give`, abgezogen wird nur, was
  der Nachbar wirklich genommen hat.
- Arme sind sechs Block-States (`down`..`east`) wie bei Rohren.
- Energieerhaltung: `transmitter::tests::a_line_of_mixed_cables_conserves_energy_every_tick` und
  `a_full_sink_backs_the_line_up_without_loss` pruefen die Summe in jedem Tick.

**Frage: Kann ein Native-Mod Energie in fremde Block Entities schieben?** Nein, bei Horizon `fdc87ae`.
`native_energy*` sind `pub(in crate::game)`, `Host` hat keinen Energieaufruf
(siehe `docs/upstream/horizon-energy-host.md`). Folge: Kabel erreichen nur Mekanism-Bloecke,
die `energy::acceptor` kennt. Fremde Mods (RF/FE-Bloecke) bleiben unerreichbar.

Abweichungen von Mekanism:

- Kein gemeinsames Netz mit einem Puffer; jedes Kabel hat einen eigenen Puffer (Ausgleich pro Tick, nicht sofort).
- Generatoren (`generators.rs`, nicht meine Datei) geben nur nach vorn und nach unten ab. Ein Kabel muss dort anliegen.
- Keine Seitenkonfiguration und kein Configurator fuer Kabel.

### Task B: Logistical, Restrictive und Diversion Transporter (TODO M08)

Sechs Bloecke: vier Tiers `*_logistical_transporter`, `mekanism:restrictive_transporter`, `mekanism:diversion_transporter`.

- Werte aus `TransporterTier.java`: Pull 1 / 16 / 32 / 64 Items, Speed 5 / 10 / 20 / 50. Ein Stack bekommt je Tick
  `speed` Fortschritt und wird bei 100 weitergegeben (Verweildauer `ceil(100/speed)` Ticks je Block).
- Wegkosten wie `getCost`: Ultimate-Speed geteilt durch Tier-Speed (verdoppelt, ganzzahlig: 20/10/5/2).
  Restrictive kostet 1.000 (hier 2.000, gleiches Verhaeltnis), Diversion ist Basic.
- Diversion: je Seite Disabled / High / Low (`DiversionControl`). High braucht ein Redstone-Signal, Low keines.
  Rechtsklick mit leerer Hand schaltet die angeklickte Seite weiter.
- Aufbau: Block Entity mit 8 Slots. Slot 0 ist der Eingang (Hopper, `insert_item`), Slots 1..7 enthalten Stacks
  unterwegs. Fortschritt, Eintrittsseite und Herkunft stehen in `mekanism_transit` (IntArray) und werden mit gespeichert.
- Weitergabe nur, wenn der naechste Halter den Stack genommen hat: ins naechste Transporter-Entity per
  `change_mod_block_entity`, in ein Inventar per `insert_item` (der Rest bleibt). Verlustfrei bei Speichern, Laden,
  Chunk-Entladen. Beim Abbau gibt der Container seine Slots selbst ab (`docs/native-mods.md`, Abschnitt Container).
- Wegsuche: Dijkstra ueber die Transporterbloecke (Limit 512 Bloecke), billigster Weg zu einem Inventar. Gleiche
  Kosten gehen an die niedrigere Seitennummer. Das Inventar, aus dem gezogen wurde, ist nie Ziel. Ein Inventar, das
  den Stack ablehnte, wird fuer diesen Stack uebersprungen. Zurueck geht ein Stack nur, wenn nichts anderes fuehrt.
  Ohne Ziel wartet er und versucht es spaeter erneut.
- Tests (`logic/energy`, `transporter.rs`): Erhaltung der Itemsumme in jedem Tick, Verweildauer, billigster Weg,
  Restrictive wird gemieden, Gleichstand, voller Zielblock faellt auf den naechsten zurueck, Diversion/Seitenmodi, Speicherformat.

Abweichungen von Mekanism:

- **Pull-Regel:** Mekanism zieht per Configurator. Dieses Repo hat den Configurator nicht. Ersatz: Schleichen +
  Rechtsklick mit leerer Hand schaltet den Verbindungsmodus der angeklickten Seite (Normal, Push, Pull, None) weiter;
  Meldung in der Action Bar. Pull holt alle 10 Ticks `pull` Items.
- Kein `container_slots` im Host (nur Gameplay-intern). Darum kann man Inventare nicht auflisten: Ziel-Annahme
  laesst sich nicht vorab pruefen (`insert_item` ist ein echtes Einfuegen), und Pull durchsucht blind Slots 0..53 mit `extract_item`.
- Kein Netzwerk mit Pfadvorhersage, keine Farben, keine Idle-Pfade, kein Stack-Zusammenfassen.
  Zwei Transporter, die beide voll sind und Stacks tauschen wollen, koennen sich blockieren (nichts geht verloren).
- Die Standardverbindung ist Normal (kein Pull), wie bei Mekanism.

### Nicht gemacht

- **Logistical Sorter** (`mekanism:logistical_sorter`): nicht begonnen. Fehlt noch.
- Kein Native-Kompiliercheck: `horizon-launcher native-build` war gesperrt. Alles in `transmitters.rs`,
  `transporters.rs` und den `lib.rs`-Anpassungen ist **nicht kompiliert**. Es folgt den vorhandenen Mustern (`bins.rs`, `pumps.rs`) und den
  Signaturen aus dem generierten `Host`; Fehler beim ersten echten Build sind wahrscheinlich nur kleine Typfehler.

## Pruefungen

| Pruefung | Ergebnis |
|---|---|
| `cargo test --manifest-path logic/energy/Cargo.toml` | 149 Tests ok |
| `python scripts/validate_content.py` | OK, 5763 JSON-Dateien |
| `python scripts/parity.py --check` | 0 Regressionen |
| `cargo horizon-mod build` / `check` (Task A, Inhalt) | ok |
| Native Kompilierung | nicht ausgefuehrt (gesperrt) |

## Ingame-Checkliste

Kabel:
1. Energiewuerfel ausgeben, Kabel anlegen: Arme erscheinen zu Wuerfeln, Maschinen, Pumpen und Kabeln.
2. Solargenerator vorn oder unten an ein Kabel, Kabel zu einem Wuerfel: Wuerfel laedt, Kabel leeren sich.
3. Basic-Kabel nach 8.000 gefuellt, Advanced mit hoeherem Durchsatz.
4. Kabel abbauen und neu setzen: kein Fehler.

Transporter:
1. Transporter zwischen Truhe A und Truhe B legen. Schleichen + Rechtsklick (leere Hand) auf die Seite zu A bis "Pull".
   Items wandern nach B (Basic: 1 Item je 10 Ticks).
2. Tier-Geschwindigkeit: Ultimate deutlich schneller als Basic.
3. Voller Zielblock: Items gehen in die zweite Truhe; keine Items verschwinden.
4. Restrictive Transporter wird nur genutzt, wenn es keinen anderen Weg gibt.
5. Diversion: Rechtsklick (leere Hand) stellt eine Seite auf High; ohne Redstone sperrt sie, mit Signal oeffnet sie.
6. Transporter mit Items abbauen: Items fallen heraus. Welt speichern und laden: Items laufen weiter.
7. Hopper unter/ueber einem Transporter steckt Items hinein.

## Offene Punkte

- Logistical Sorter und Filter.
- Configurator (Pull/Push per Werkzeug).
- Gemeinsame Kabelnetze mit einem Puffer; Energie in fremde Mods (Host-Aufruf fehlt).
- Stack-Zusammenfassen und Deadlock-Vermeidung zwischen vollen Transportern.
- Native-Kompilierung nachholen.
