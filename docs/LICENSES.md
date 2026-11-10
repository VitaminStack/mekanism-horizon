# Lizenzen und Attribution

Prüfung vom 10.11.2026 (TODO M13: „Lizenz, Copyright und Asset-Attribution vor Releases prüfen“). Keine Rechtsberatung. Ergebnis: Die Grundlage ist sauber (alles MIT), aber es gibt Lücken, die vor einem Release geschlossen werden sollten.

## Was vorhanden ist

| Datei / Bereich | Lizenz | Befund |
|---|---|---|
| `LICENSE` (Repository-Wurzel) | MIT, `Copyright (c) 2026 SozialHilfe` | vorhanden |
| `content/mekanism/LICENSE-MEKANISM` | MIT, `Copyright (c) 2017-2025 Aidan C. Brady` | vorhanden |
| `content/mekanismgenerators/LICENSE-MEKANISM` | identisch (per `cmp` geprüft) | vorhanden |
| `content/mekanismtools/LICENSE-MEKANISM` | identisch | vorhanden |
| `content/mekanismadditions/LICENSE-MEKANISM` | identisch | vorhanden |
| `logic/energy/Cargo.toml`, `native/mekanism_native/api/Cargo.toml` | `license = "MIT"` | vorhanden |

Mekanism steht unter MIT, daher ist die Weitergabe von Code, Texturen, Modellen und Sounds daraus erlaubt, solange Copyright-Hinweis und Lizenztext mitgehen. Die vier `LICENSE-MEKANISM` liegen im Quellordner jeder Content-Mod.

## Herkunft der Inhalte je Content-Mod

| Mod | Aus Mekanism übernommen (Skript) | Eigen |
|---|---|---|
| `mekanism` | Texturen (188 PNG), Modelle, Blockstates, Rezepte, Tags, Namen, 10 Sounds (`import_upstream.py`, `import_energy_cube.py`) | Registry-Deklarationen, Loot, Worldgen (`gen_resources.py`), Debug-Blöcke |
| `mekanismgenerators` | Modelle, Texturen (8 PNG), 4 Sounds (`import_upstream.py`, `import_generator_models.py`) | Registry, Loot |
| `mekanismtools` | Texturen (121 PNG), Equipment-Assets, Rezepte, Tags, Namen (`gen_tools.py`) | Material-Tabellen aus Mekanisms Standardwerten, Komponenten |
| `mekanismadditions` | Modelle, Blockstates, Loot, Rezepte, Tags, Namen; Texturen aus Upstream-Graustufen **vorgefärbt** (`gen_additions.py`, Pillow) | Block-Deklarationen |
| `native/mekanism_native`, `logic/energy` | Werte (Tier-Kapazitäten, Rezeptzeiten, Energiekosten) nach Mekanism-Quelltext; Rust-Code selbst geschrieben | alles Code |

## Lücken und Empfehlungen

1. **Kein Mekanism-Hinweis im Wurzel-`LICENSE`, `README` und in den Mod-Archiven.** `LICENSE-MEKANISM` liegt nur im Quellordner. Ob `cargo horizon-mod build` die Datei in die `.hmod` packt, ist nicht geprüft. Prüfen und notfalls Hinweis in `description` bzw. Release-Notizen aufnehmen. Ein `NOTICE`/„Danksagung“-Abschnitt in `README.md` fehlt (README nennt Mekanism nur als Vorlage).
2. **Copyright-Zeitraum.** Die kopierte Lizenz nennt `2017-2025`. Upstream-Branch 26.3 kann ein neueres Jahr tragen. Beim nächsten Upstream-Abgleich `LICENSE` neu kopieren (`import_upstream.py` liest sie aus dem Checkout) und vergleichen.
3. **Inhaberangaben uneinheitlich.** `LICENSE` nennt `SozialHilfe`, `horizon-mod.toml` und das GitHub-Repository `VitaminStack`. Inhaber klären und einheitlich eintragen.
4. **Drittbeiträge in Mekanism.** Mekanism-Assets stammen teils von Mitwirkenden (Texturen, Sounds, Modelle). Das Mekanism-Repository deklariert MIT für das Gesamtprojekt. Pro-Datei-Herkunft (z. B. Sounds, Blockbench-Modelle der Generatoren) ist hier nicht einzeln belegt. Vor einem Release die Upstream-Hinweise zu Assets prüfen (README/Credits/`docs`) und bei abweichenden Lizenzen die Datei entfernen oder ersetzen.
5. **Abgeleitete Texturen.** Die Additions-Texturen sind farbige Ableitungen. Das ist unter MIT zulässig, sollte aber im Mod-Hinweis als „abgeleitet aus Mekanism-Texturen“ stehen.
6. **Fehlende Attribution im Spiel.** Weder Mod-Beschreibungen (`horizon-mod.toml`) noch die Mod-Liste nennen Mekanism-Autoren. Die Beschreibung sagt „Unofficial“. Empfehlung: Satz „Basiert auf Mekanism (MIT, Aidan C. Brady und Mitwirkende)“ ergänzen.
7. **Horizon-Abhängigkeit.** `native/.../api/Cargo.toml` bindet `horizon-client` aus `horizon-rust/horizon` ein. Deren Lizenz ist hier nicht geprüft. Für Release-Artefakte, die Horizon-Quellcode enthalten oder daraus gebaut sind, die Horizon-Lizenz prüfen.
8. **Horizon-Entwürfe.** `docs/upstream/*.md` sind eigene Textentwürfe für Horizon, keine Kopien. Unkritisch.
9. **Marken.** „Mekanism“ ist der Projektname des Originals. Der Port ist inoffiziell (README sagt das). Beibehalten und in Release-Seiten wiederholen.
10. **Rust-Abhängigkeiten.** `logic/energy` hat eine `Cargo.lock`. Abhängigkeitslizenzen nicht ausgewertet (z. B. mit `cargo deny` / `cargo license` prüfen).

## Checkliste vor einem Release

- [ ] Inhaber im `LICENSE` klären (Punkt 3).
- [ ] `LICENSE-MEKANISM` gegen aktuellen Upstream abgleichen (Punkt 2).
- [ ] Prüfen, ob die Lizenzdatei in der `.hmod` landet, sonst beilegen (Punkt 1).
- [ ] Attribution in README und Mod-Beschreibungen ergänzen (Punkte 1, 6).
- [ ] Asset-Herkunft stichprobenartig gegen Upstream prüfen (Punkt 4).
- [ ] Horizon-Lizenz und Rust-Abhängigkeiten prüfen (Punkte 7, 10).
