# Erzverarbeitung (M06)

Die Kette von Mekanisms 5x-Verarbeitung, wie sie hier läuft. Menge und Zeit der Maschinen stehen in
`native/mekanism_native/api/src/machine.rs` (Werte aus Mekanism 26.3), die Rezepte in
`content/mekanism/data/mekanism/recipe/processing/<Metall>/`.

## Kette (Eisen, 1 Erz)

| Schritt | Maschine | Rezepttyp | Ein | Aus |
|---|---|---|---|---|
| 1 | Chemical Dissolution Chamber (16 FE/t, 100 Ticks) | `dissolution` | 1 Erz + 1 mB Schwefelsäure je Tick (100 mB) | 1.000 mB schmutziger Schlamm (`dirty_iron`) |
| 2 | Chemical Washer (5 FE/t, 1 Durchlauf je Tick) | `washing` | 1 mB schmutziger Schlamm + 5 mB Wasser | 1 mB sauberer Schlamm (`clean_iron`) |
| 3 | Chemical Crystallizer (15 FE/t, 200 Ticks) | `crystallizing` | 200 mB sauberer Schlamm | 1 Kristall (`crystal_iron`) |
| 4 | Chemical Injection Chamber | `injecting` | 1 Kristall + 1 mB Chlorwasserstoff je Tick | 1 Splitter (`shard_iron`) |
| 5 | Purification Chamber | `purifying` | 1 Splitter + 1 mB Sauerstoff je Tick | 1 Klumpen (`clump_iron`) |
| 6 | Crusher | `crushing` | 1 Klumpen | 1 schmutziger Staub (`dirty_dust_iron`) |
| 7 | Enrichment Chamber | `enriching` | 1 schmutziger Staub | 1 Staub (`dust_iron`) |
| 8 | Ofen / Blast Furnace (Energized Smelter fehlt, siehe unten) | `smelting` | 1 Staub | 1 Barren |

Aus 1.000 mB Schlamm werden 5 Kristalle und am Ende **5 Barren je Erz** (Vanilla: 1; Enrichment-Weg: 2).
Roherz-Blöcke geben 6.000 mB Schlamm bei 2 mB Säure je Tick. Ingame getestet am 11.11.2026
([TESTLOG.md](TESTLOG.md), Abschnitt „Dissolution Chamber, Washer, Crystallizer und Erzkette“).
Gold, Kupfer, Zinn, Blei, Osmium und Uran haben dieselben Rezepte (je 7 Metalle: Kristall, Splitter, Klumpen,
schmutziger Staub); Uran gibt zusätzlich Flusssäure-Rezepte, die erst mit der Pressurized Reaction Chamber laufen.

## Schwefelsäure

Die Säure kommt wie im Original aus dem Chemical Infuser (Schwefeltrioxid + Wasserdampf), Schwefeltrioxid aus
Schwefeldioxid + Sauerstoff, Schwefeldioxid aus Schwefelstaub im Chemical Oxidizer, Wasserdampf aus Wasser im
Rotary Condensentrator (Modus Fluid zu Chemical), Sauerstoff aus dem Electrolytic Separator. Alle Stufen
sind einzeln ingame getestet; die volle Kette von Wasser bis Säure ist es **nicht**: der Test füllte
Dissolution und Washer per `/data merge` (Säure, Wasser).

## Chemie und Fluids in diesen Maschinen

- Dissolution: Tank 0 (Säure, 10.000 mB, füllbar von Nachbarn und Pipes), Tank 1 (Schlamm, 10.000 mB, wird in alle Nachbarn ausgeworfen).
  Die Säure ist ein Tank, kein Puffer wie bei Purification/Injection (`Machine::chemical_tank`).
- Washer: Tank 0 Wasser, Tank 1 schmutziger Schlamm, Tank 2 sauberer Schlamm (je 10.000 mB), `machine::wash`.
- Crystallizer: ein Eingangs-Tank (10.000 mB), Item im Output-Slot.

## Grenzen (Horizon / noch nicht gebaut)

- **Energized Smelter** blockiert (Horizon liefert Native-Mods keine `minecraft:smelting`-Rezepte, siehe TODO M05); Schritt 8 geht über einen Vanilla-Ofen.
- Keine Pipes/Tubes: die Kette läuft über direkt benachbarte Maschinen (Auswurf in Nachbarn) oder per Hand.
- Kein Wasser-Eimer für Maschinen (Horizons Eimer-Hook gilt nur für Tanks): Wasser kommt aus einem Fluid Tank neben dem Washer.
- Chemie-Item-Slots (Tanks per Item füllen/leeren), Energie-Slot, Upgrades, Side-Config, Redstone fehlen wie bei den anderen Chemie-Maschinen.
- Ein Tank nimmt jedes Chemical, das in einem Rezept der Maschine vorkommt (Horizons `accepts`-Hook kennt weder Position noch anderen Tank): ein falscher Schlamm im Washer-Eingang blockiert, bis er geleert ist.
- Rezeptwechsel (Erz-Sorte) bei halbem Fortschritt: Dissolution und Crystallizer behalten bzw. setzen den Fortschritt nach dem Original-Muster, nicht gesondert getestet.
- Crafting der drei Maschinen (Dissolution/Washer: Chemical/Fluid Tank, Crystallizer: Fluorit, Ultimate Circuit) ist per Import vorhanden, nicht ingame geprüft; `mek_data` wird zu normalem Shaped Crafting.
