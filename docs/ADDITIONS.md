# Mekanism: Additions (M12)

Content-Mod `content/mekanismadditions` (Namespace `mekanismadditions`). Rein Content, kein nativer Code: jeder Block ist eine Vanilla-Vorlage (`template`) plus `properties`.

Erzeugt mit `python -I scripts/gen_additions.py <Mekanism-26.3-Checkout>` (Sparse-Clone von `src/additions`, `src/datagen/generated/mekanismadditions`, `src/api/java/mekanism/api/text` und `LICENSE` reicht; Pillow wird gebraucht). Modelle, Blockstates, Item-Definitionen, Loot-Tabellen, Rezepte, Tags und Namen stammen aus dem Checkout. Lizenz: MIT, siehe `content/mekanismadditions/LICENSE-MEKANISM`.

## Umfang (271 Blöcke)

18 Farben (Vanilla-Farben plus `aqua` und `dark_red`), je 15 Varianten: Plastic, Slick, Glow, Reinforced, Road, Transparent, Stairs, Slabs, Fences, Fence Gates, Glow Stairs/Slabs, Transparent Stairs/Slabs und Glow Panel; dazu Obsidian TNT.

| Variante | Vorlage | Werte |
|---|---|---|
| Plastic, Road | `stone` | Härte 5, Widerstand 6 |
| Slick | `stone` | `friction` 0,98 |
| Glow, Glow Stairs/Slabs | `stone`, `stone_stairs`, `stone_slab` | Licht 10 |
| Reinforced | `stone` | Härte 50, Widerstand 1200 |
| Transparent (+ Stairs/Slabs) | `glass`, `stone_stairs`, `stone_slab` | `occludes: false` |
| Fence / Gate | `nether_brick_fence` / `oak_fence_gate` | Härte 5, Widerstand 6 |
| Glow Panel | `small_amethyst_bud` | Licht 15, eigene `shapes` je `facing` aus `BlockShapes.GLOW_PANEL` |
| Obsidian TNT | `tnt` | Härte 0 |

Alle: Sound Stein, Kartenfarbe wie `EnumColor`, kein Werkzeug nötig.

Farben: Upstream färbt die grauen Texturen zur Laufzeit (Block-/Item-Farbanbieter). Horizon-Content hat keinen Farbanbieter. Der Generator färbt deshalb jede Textur je Farbe vor (`textures/block/<farbe>/`) und erzeugt je Farbe eigene Modelle (`models/block/<farbe>/`); die gemeinsamen `colored/*`-Modelle sind ohne `tintindex` kopiert. Beim Glow Panel ist nur die Mittelplatte gefärbt, wie im Original.

## Rezepte

Übernommen: Slabs, Stairs, Fences, Gates, Roads, Reinforced, Glow, Transparent (Shaped/Shapeless) und die Slick-Rezepte der Enrichment Chamber, soweit alle Mekanism-Zutaten registriert sind. Nicht enthalten:

- Plastic-Grundrezept und Glow Panels: brauchen `mekanism:hdpe_sheet` (HDPE, M06 Chemie). Der Generator nimmt sie automatisch auf, sobald das Item in `scripts/registry.json` steht.
- Umfärben (`neoforge:difference`-Zutat) und Painting-Machine-Rezepte (`mekanism:painting`, M06).
- Pigment-Extraktion, Balloons, Walkie-Talkie.

## Abweichungen vom Original (Horizon-Grenzen, nichts vorgetäuscht)

| Original | Hier | Grund |
|---|---|---|
| Obsidian TNT: eigene Entität, Radius 12, Zündzeit konfigurierbar | explodiert wie Vanilla-TNT (Radius 4, Zündzeit 80 Ticks) | Eigene Entität/Explosion braucht Code. Ein Content-Block kann nur die Vorlage `tnt` übernehmen. Ein nativer Hook für Explosionsstärke fehlt |
| Plastic Road: Geschwindigkeitsschub (`stepOn`) | normaler Block | `stepOn` ist Code; es gibt keine Content-Eigenschaft für Geschwindigkeit |
| Glow/Transparent: `emissiveRendering`, CTM-Texturen, blendet verdeckte Innenflächen gleicher Farbe aus (`skipRendering`) | Licht über `light`; keine CTM, keine Face-Auslassung | Rendercode/Mod nicht verfügbar |
| Transparent: `isValidSpawn`/`isSuffocating`/`isViewBlocking` aus, Leuchtfeuerfarbe | Glas-Vorlage (Spawn und Leuchtfeuer wie Glas) | Nicht als Eigenschaft ladbar |
| Fence/Slab/Obsidian TNT: Extended-Fluid-Logging (Lava, Wasser) | Wasser wie Vanilla | `IStateExtendedFluidLoggable` ist Code |
| Glow Panel: eigene Blockklasse, Platzierungsregeln | Vorlage `small_amethyst_bud` (6 Richtungen, wasserfähig, fällt ohne Stütze ab) | Verhalten wie Original ähnlich, Mindestgröße der Stützfläche nicht identisch |
| Konfiguration (TNT-Radius/Zündzeit, Baby-Mobs, Spawn-Raten) | fehlt | Horizon-Content hat keine Config |
| Advancements, Rezeptbuch-Einträge | fehlt | Datentyp nicht geladen |

## Nicht gebaut: Entities, Balloons, Walkie-Talkies, Spawn Eggs

Nur untersucht (Schema aus `cargo horizon-mod`, kein Quelltext von Horizon verfügbar):

- Horizon-Content kann `entity_type/*.json` deklarieren (Vorlage + Attribute, „living type“), also eine Vanilla-Kreatur mit Vanilla-KI und Vanilla-Modell unter neuer ID. Eigenes Verhalten, eigene Modelle, Biom-Spawns und Netzwerkdaten sind nicht Teil des Schemas.
- **Baby-Mobs** (Creeper, Enderman, Skeleton, Stray, Wither Skeleton, Bogged, Parched): Original sind eigene Entitätsklassen mit kleinem Modell und Mekanisms Drops/Spawn-Biomen (`biome_modifier`). Als Vorlage nur mit Vanilla-Baby-Variante möglich; Spawns brauchen NeoForge-Biom-Modifier. Nicht gebaut, da keine Parität erreichbar.
- **Balloons**: Entität, die per Leine an Block oder Spieler hängt, steigt auf und platzt (`EntityBalloon`, Interaktion, Netzwerk-Sync, Farbe, Advancement `pop_pop`). Braucht nativen Code (Entity-Tick, Rechtsklick auf Entität, Sync); Horizon-Content hat nichts davon.
- **Walkie-Talkie**: Kanäle, Sprechfunk zwischen Spielern per Paket/Chat (`ItemWalkieTalkie`, Kanal-Komponenten, Taste). Braucht Items mit Zustand, Tastenabfrage und Netzwerk; native Mods dürfen laut Fähigkeiten „see the keys you hold…/message“ nutzen, der Aufwand gehört in einen eigenen nativen Task (M13-Interop).
- **Spawn Eggs**: setzen die Baby-Entitäten voraus.

Diese Punkte bleiben in `TODO.md` offen.
