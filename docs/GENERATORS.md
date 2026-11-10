# Mekanism: Generators, Abgleich mit Upstream (M12)

Vergleich von `content/mekanismgenerators` mit Mekanism Generators 26.3 (Upstream-Commit `31bb4906`). Alle Assets, Rezepte und Tags kommen aus `scripts/import_upstream.py` und `scripts/import_generator_models.py`; die Registrierung steht in `scripts/gen_resources.py` (Abschnitt Generators).

Reihenfolge nach Änderungen: `gen_resources.py`, `import_upstream.py <Mekanism>`, `import_generator_models.py <Mekanism>`, `scripts/Build-Content.ps1`.

## Übernommen (Content, ohne Funktion außer dem Block selbst)

| Upstream-ID | Stand |
|---|---|
| `heat_generator`, `solar_generator`, `bio_generator`, `advanced_solar_generator`, `wind_generator` | funktionieren (nativ, M04), Rezepte, Modelle, Loot, Lang |
| `solar_panel` | Item, Rezept |
| `turbine_blade`, `hohlraum` | nur Item, Rezept (Hohlraum: Metallurgic Infuser, Rezept im Namespace `mekanismgenerators`); Hohlraum speichert keine Chemikalien |
| `turbine_casing`, `turbine_rotor` (Modell `fence_post`), `fission_reactor_casing`, `fusion_reactor_frame`, `reactor_glass`, `laser_focus_matrix`, `electromagnetic_coil`, `rotational_complex`, `saturating_condenser`, `fission_fuel_assembly`, `control_rod_assembly` | nur Block: Modell, Textur, Loot, Lang, Rezept. Ohne Block-Entity und ohne Multiblock-Logik. Turbine Blade lässt sich nicht in den Rotor stecken |

Block-Stärken wie upstream: Casings/Frame/Glas 5 / 9, übrige 3,5 / 16, Laser Focus Matrix 3,5 / 4,8.

## Blockiert oder aufgeschoben (braucht nativen Code)

| Upstream-ID | Grund |
|---|---|
| Industrial Turbine (Multiblock) mit `turbine_valve`, `turbine_vent` | Formung, Dampf, Rotor-/Blattzahl, Kondensatoren, Energieabgabe. Valve und Vent noch nicht registriert, weil ohne Multiblock ohne Funktion |
| Fission Reactor mit `fission_reactor_port`, `fission_reactor_logic_adapter` | Multiblock, Kühlmittel/Brennstoff/Abfall, Hitze, GUI |
| Fusion Reactor mit `fusion_reactor_controller`, `fusion_reactor_port`, `fusion_reactor_logic_adapter` | Multiblock, Plasma, Laser, Hohlraum-Zündung. Der Rahmen ist craftbar nur, wenn `#c:pellets/polonium` existiert (fehlt, M07/Radioaktivität) |
| `gas_burning_generator` | Rezept braucht `mekanism:electrolytic_core` (M06 Electrolytic Separator); Maschine braucht Gas-Verbrennung, nativ |
| Chemikalien `deuterium`, `tritium`, `fusion_fuel`, Rezept `chemical_infusing/fusion_fuel` | Chemikalien nur im nativen Chemie-Tisch (`gen_chemicals.py` kennt nur den Namespace `mekanism`); Tritium/Deuterium-Rezepte brauchen Rotary Condensentrator, Electrolytic Separator, Solar Neutron Activator |
| Eimer und Fluide `bioethanol`, `deuterium`, `tritium`, `fusion_fuel` | Horizon kennt nur Vanilla-Fluide (siehe `docs/CHEMISTRY.md`) |
| Module `geothermal_generator_unit`, `solar_recharging_unit` | Mekanism-Modulsystem fehlt |
| Superheating Element (Blockstate upstream, kein Registrierungseintrag) | nicht registriert |
| Schadensarten (`damage_type`), Advancements, EMI-Aliase | nicht übernommen (nativ bzw. ohne Gegenstück); die Strukturtags für die Multiblock-Prüfung kopiert der Importer mit |

Namespace und Abhängigkeit: Der Mod `mekanismgenerators` nutzt Texturen (`mekanism:block/models/ports`), Tags und Rezeptzutaten aus `mekanism`; `mekanism` muss mitgeladen sein. Das Manifest hat kein Abhängigkeitsfeld, nur den Hinweis in der Beschreibung. Beide Mods in `horizon-mods.txt` einschalten.
