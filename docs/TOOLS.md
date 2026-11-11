# Mekanism: Tools (M12)

Content-Mod `content/mekanismtools` (Namespace `mekanismtools`, braucht `mekanism` wegen der Barren). Rein Content, kein nativer Code: Horizon-Items sind eine Vanilla-Vorlage (`template`) plus `components`, und Werkzeugwerte, Rüstung, Schild und Speer sind Vanilla-Datenkomponenten.

Erzeugt mit `python scripts/gen_tools.py <Mekanism-26.3-Checkout>` (Sparse-Clone von `src/tools` und `src/datagen/generated/mekanismtools` reicht). Die Materialtabellen im Skript sind Mekanisms Standardwerte (`*MaterialDefaults`, `ToolsMaterialConfig`); Assets, Rezepte, Tags und Namen werden aus dem Checkout kopiert. Lizenz: MIT, siehe `content/mekanismtools/LICENSE-MEKANISM`.

## Umfang (91 Items)

- Paxel: Wood, Stone, Copper, Iron, Gold, Diamond, Netherite (Haltbarkeit = 2x Vanilla, Schaden = Axtbasis + 1).
- Je Bronze, Lapis Lazuli, Osmium, Refined Glowstone, Refined Obsidian, Steel: Axe, Hoe, Pickaxe, Shovel, Sword, Paxel, Spear, Shield, Helmet, Chestplate, Leggings, Boots, Horse Armor, Nautilus Armor.
- Haltbarkeit, Abbaugeschwindigkeit, Angriffswerte, Verzauberbarkeit, Reparaturmaterial (`#c:ingots/...`), Rüstung, Zähigkeit, Rückstoßresistenz, Feuerfestigkeit (Refined Obsidian, Netherite-Paxel), Speerwerte und Schildwerte (Blockverzögerung, Schadensschwelle) stehen als Komponenten in `data/mekanismtools/horizon/item/`.
- Mining-Stufe über die Block-Tags `mekanismtools:incorrect_for_<material>_tool` (Bronze/Osmium: Eisen, Lapis: Stein, Steel/Refined Glowstone: Diamant, Refined Obsidian: Netherite).
- 107 Rezepte (Werkzeuge, Rüstung, Schilde, Paxel, Barren-Schmelzen), Tags (`c:tools/paxel`, `minecraft:pickaxes`, `piglin_safe_armor` ...), Rüstungs-Equipment-Assets mit Texturen, Items-Modelle, `en_us`/`de_de`-Namen.

## Abweichungen vom Original (Horizon-Grenzen, nichts vorgetäuscht)

| Original | Hier | Grund |
|---|---|---|
| Paxel wirkt als Axt **und** Schaufel (`PaxelItem.useOn`) | nur Axt-Aktion (Rinde schälen); Grasflächen werden nicht zu Pfaden | Content-Items haben keinen Code; `minecraft:block_transformer` nimmt eine Transformation. Braucht einen nativen Hook; `use_item_on` (API 13) existiert, siehe [ITEM_HOOKS.md](ITEM_HOOKS.md), ist hier aber noch nicht gebaut |
| `mekanismtools:paxel`-Rezept mittelt die Haltbarkeit der Zutaten | `minecraft:crafting_shaped`, Paxel startet unbeschädigt | Kein eigener Rezepttyp in Horizon-Content |
| Schild mit eigener Textur (`mekanismtools:shield` Special-Model) | Schild zeigt das Vanilla-Schild-Modell; Werte (Haltbarkeit, Blockverzögerung, Schwelle) stimmen | Eigener Item-Model-Typ ist NeoForge/Mekanism-Code |
| Rüstungs-Trims mit Mekanism-Paletten (`neoforge:trimmed_armor`, `trim_overrides`) | Vanilla-Modell, keine Mekanism-Palette | NeoForge-only |
| `ToolsDataComponents.DISPLAY_HP`, Haltbarkeits-Tooltip | fehlt (Vanilla-Haltbarkeitsleiste vorhanden) | Eigene Komponente braucht Code |
| Mob-Spawn-Ausrüstung (`MobEquipmentHelper`), Konfiguration, Advancements, Rezeptbuch-Aliase | fehlt | Code bzw. Datentypen, die Horizon-Content nicht lädt |
| Konfigurierbare Materialwerte | feste Standardwerte | Horizon-Content hat keine Config; Werte per Neugenerierung ändern |

Ob Horizon die Items für Rüstungs-/Schildwerte im Multiplayer-Server identisch liest, ist nicht getestet.
