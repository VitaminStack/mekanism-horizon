"""Generate the Horizon-specific half of Mekanism's resource content (M02):
registry declarations (`data/<mod>/horizon/`), block loot and ore
generation, for both content mods: `content/mekanism` and, as upstream ships
the generators as their own mod, `content/mekanismgenerators` (M04). Assets, recipes, tags and names come from upstream through
`scripts/import_upstream.py`, which runs after this one.

IDs and ore-generation defaults follow Mekanism 26.3 (`MekanismItems`,
`MekanismBlocks`, `OreType`, `WorldConfig`). Run: `python scripts/gen_resources.py`.
"""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[1]
NS = 'mekanism'
GEN = 'mekanismgenerators'
REGISTRY = ROOT / 'scripts' / 'registry.json'

PRIMARY = ['osmium', 'tin', 'lead', 'uranium']
ALLOYS = ['bronze', 'steel', 'refined_obsidian', 'refined_glowstone']
DUSTS = ['osmium', 'tin', 'lead', 'uranium', 'bronze', 'steel', 'refined_obsidian', 'iron', 'gold',
         'copper', 'diamond', 'emerald', 'netherite', 'lapis_lazuli', 'coal', 'charcoal', 'quartz',
         'obsidian', 'sulfur', 'lithium', 'fluorite']
MISC_ITEMS = ['fluorite_gem', 'salt',
              'basic_control_circuit', 'advanced_control_circuit', 'elite_control_circuit',
              'ultimate_control_circuit', 'alloy_infused', 'alloy_reinforced', 'alloy_atomic',
              'enriched_carbon', 'enriched_redstone', 'enriched_diamond', 'enriched_refined_obsidian',
              'enriched_tin', 'enriched_gold', 'enriched_iron']

# registry ids in declaration order, per content mod (= namespace)
registry = {ns: {'items': [], 'blocks': []} for ns in (NS, GEN)}


def put(rel, obj, ns=NS):
    path = ROOT / 'content' / ns / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def item(id_, template='minecraft:iron_ingot', stack=64, ns=NS):
    registry[ns]['items'].append(id_)
    put(f'data/{ns}/horizon/item/{id_}.json', {'template': template, 'rarity': 'common', 'max_stack_size': stack},
        ns)


def loot(id_, drop, count=None, explosion_proof=False, ns=NS):
    entry = {'type': 'minecraft:item', 'name': f'{ns}:{drop}'}
    if count:
        entry['functions'] = [{'function': 'minecraft:set_count',
                               'count': {'type': 'minecraft:uniform', 'min': count[0], 'max': count[1]}}]
    put(f'data/{ns}/loot_table/blocks/{id_}.json', {
        'type': 'minecraft:block',
        'pools': [{'entries': [entry], 'rolls': 1} if explosion_proof else
                  {'condition': {'type': 'minecraft:survives_explosion'}, 'entries': [entry], 'rolls': 1}],
        'random_sequence': f'{ns}:blocks/{id_}',
    }, ns)


def block(id_, template, hardness, resistance, drop=None, count=None, requires_tool=True, extra=None,
          explosion_proof=False, sounds=None, ns=NS):
    registry[ns]['blocks'].append(id_)
    props = {'hardness': hardness, 'explosion_resistance': resistance, 'requires_tool': requires_tool}
    if sounds:
        # Sound type as a vanilla block id; overrides the template's (Horizon `properties.sounds`).
        props['sounds'] = sounds
    put(f'data/{ns}/horizon/block/{id_}.json', {
        'template': template,
        'properties': props,
        **(extra or {}),
        'item': True,
    }, ns)
    loot(id_, drop or id_, count, explosion_proof, ns)


# --- items ---
for m in PRIMARY:
    item(f'ingot_{m}')
    item(f'raw_{m}', 'minecraft:raw_iron')
    item(f'nugget_{m}', 'minecraft:iron_nugget')
for a in ALLOYS:
    item(f'ingot_{a}')
    item(f'nugget_{a}', 'minecraft:iron_nugget')
for d in DUSTS:
    item(f'dust_{d}', 'minecraft:sugar')
for i in MISC_ITEMS:
    item(i, 'minecraft:quartz' if i in ('fluorite_gem',) else 'minecraft:iron_ingot')

# --- blocks (hardness/resistance from MekanismBlocks / BlockResource) ---
# Sound types: Mekanism sets none on resource blocks and steel casing (vanilla default STONE), only
# SAND on salt and DEEPSLATE on deepslate ores. Templates iron_block (METAL) and clay (GRAVEL) differ.
STONE = 'minecraft:stone'
for m in PRIMARY:
    block(f'block_{m}', 'minecraft:iron_block', 5.0, 6.0, sounds=STONE)
    block(f'block_raw_{m}', 'minecraft:raw_iron_block', 5.0, 6.0)
    block(f'{m}_ore', 'minecraft:iron_ore', 3.0, 3.0, f'raw_{m}')
    block(f'deepslate_{m}_ore', 'minecraft:deepslate_iron_ore', 4.5, 3.0, f'raw_{m}')
for a in ALLOYS:
    # Refined obsidian is blast-proof in Mekanism (resistance 1200).
    block(f'block_{a}', 'minecraft:iron_block', 50.0 if a == 'refined_obsidian' else 5.0,
          1200.0 if a == 'refined_obsidian' else 6.0, sounds=STONE)
block('block_fluorite', 'minecraft:quartz_block', 5.0, 6.0)
block('fluorite_ore', 'minecraft:diamond_ore', 3.0, 3.0, 'fluorite_gem', (1, 4))
block('deepslate_fluorite_ore', 'minecraft:deepslate_diamond_ore', 4.5, 3.0, 'fluorite_gem', (1, 4))
block('block_charcoal', 'minecraft:coal_block', 5.0, 6.0)
block('block_salt', 'minecraft:clay', 0.5, 0.5, 'salt', (4, 4), requires_tool=False,
      sounds='minecraft:sand')
block('steel_casing', 'minecraft:iron_block', 3.5, 9.0, sounds=STONE)

# --- energy (M03): cubes are ticked/buffered by native/mekanism_native ---
item('energy_tablet', stack=1)
CUBE_TIERS = ['basic', 'advanced', 'elite', 'ultimate', 'creative']
# Mekanism's BlockEnergyCube: FACING (all six directions), turned toward the placer by the native mod.
# The barrel template gives that `facing` and a block that opens its menu on use
# (`data/mekanism/horizon/menu/energy_cube.json`, filled by the native mod's screen hooks).
# `port_<side>` shows the side's energy port when its side config is not "none" (set by the native mod).
# Models, blockstates and item definitions come from `scripts/import_energy_cube.py`.
CUBE_SIDES = ['down', 'up', 'north', 'south', 'west', 'east']
CUBE_STATE = {
    'state_properties': {f'port_{d}': {'type': 'boolean'} for d in CUBE_SIDES},
    'default_state': {'facing': 'north', **{f'port_{d}': 'true' for d in CUBE_SIDES}},
}
for tier in CUBE_TIERS:
    # Mekanism drops the cube even when it is blown up (no survives_explosion), with its energy.
    block(f'{tier}_energy_cube', 'minecraft:barrel', 2.0, 2.4, extra=CUBE_STATE, explosion_proof=True)

# --- machines (M05): ticked, powered and drawn by native/mekanism_native ---
# Metallurgic Infuser: BlockTile.defaultProperties (strength 3.5, 16). The barrel template gives
# `facing` and a block that opens its menu; the native mod turns the front (north in Mekanism's
# model) toward the placer, horizontally only. Slots: 0 infusion item, 1 input, 2 output (results).
# Mekanism's energy slot is not ported (see native/mekanism_native/api/src/infuser.rs).
block('metallurgic_infuser', 'minecraft:barrel', 3.5, 16.0,
      extra={'default_state': {'facing': 'north'}})
put(f'data/{NS}/horizon/menu/metallurgic_infuser.json', {'slots': 3, 'data_slots': 6, 'results': [2]})
# Upstream has no blockstate file for machines (its client code builds it); same model per facing.
FACING_Y = {'north': 0, 'east': 90, 'south': 180, 'west': 270, 'up': 0, 'down': 0}


def facing_blockstate(id_, ns=NS):
    put(f'assets/{ns}/blockstates/{id_}.json', {'variants': {
        f'facing={f}': {'model': f'{ns}:block/{id_}', **({'y': y} if y else {})}
        for f, y in FACING_Y.items()}}, ns)


facing_blockstate('metallurgic_infuser')

# --- generators (M04), content mod `mekanismgenerators`: ticked, powered and drawn by
# native/mekanism_native. BlockTile.defaultProperties (3.5, 16) as the infuser; the barrel template
# gives `facing` and the menu, the native mod turns the front toward the placer, horizontally.
# Heat Generator slot 0: fuel (lava bucket or furnace fuel). The Solar Generator has no slot of its
# own (one hidden placeholder). Mekanism's energy (charging) slot is not ported on either.
# Wind and Advanced Solar Generator are multiblocks (bounding blocks): not yet.
MACHINE_STATE = {'default_state': {'facing': 'north'}}
block('heat_generator', 'minecraft:barrel', 3.5, 16.0, extra=MACHINE_STATE, ns=GEN)
block('solar_generator', 'minecraft:barrel', 3.5, 16.0, extra=MACHINE_STATE, ns=GEN)
item('solar_panel', ns=GEN)
put(f'data/{GEN}/horizon/menu/heat_generator.json', {'slots': 1, 'data_slots': 7, 'results': []}, GEN)
put(f'data/{GEN}/horizon/menu/solar_generator.json', {'slots': 1, 'data_slots': 5, 'results': [0]}, GEN)
for gid in ('heat_generator', 'solar_generator'):
    facing_blockstate(gid, GEN)

# Debug blocks for testing the cubes (TODO N08), ticked by native/mekanism_native.
for did, tex in (('debug_energy_source', 'redstone_block'), ('debug_energy_sink', 'coal_block')):
    block(did, 'minecraft:iron_block', 1.0, 1.0, requires_tool=False)
    put(f'assets/{NS}/models/block/{did}.json',
        {'parent': 'minecraft:block/cube_all', 'textures': {'all': f'minecraft:block/{tex}'}})
    put(f'assets/{NS}/blockstates/{did}.json', {'variants': {'': {'model': f'{NS}:block/{did}'}}})
    put(f'assets/{NS}/items/{did}.json', {'model': {'type': 'minecraft:model', 'model': f'{NS}:block/{did}'}})

# --- ore generation: OreType defaults, translated to vanilla features ---
# (name, per chunk, discard on air, max vein size, shape, min anchor, max anchor, plateau)
ORE_GEN = {
    'tin': [('small', 14, 0, 4, 'trapezoid', {'absolute': -20}, {'absolute': 94}, 0),
            ('large', 12, 0, 9, 'trapezoid', {'absolute': -32}, {'absolute': 72}, 0)],
    'osmium': [('upper', 65, 0, 7, 'trapezoid', {'absolute': 72}, {'below_top': -24}, 8),
               ('middle', 6, 0, 9, 'trapezoid', {'absolute': -32}, {'absolute': 56}, 0),
               ('small', 8, 0, 4, 'uniform', {'above_bottom': 0}, {'absolute': 64}, 0)],
    'uranium': [('small', 4, 0, 4, 'trapezoid', {'above_bottom': 0}, {'absolute': 8}, 0),
                ('buried', 7, 0.75, 9, 'trapezoid', {'above_bottom': -24}, {'above_bottom': 56}, 16)],
    'fluorite': [('normal', 5, 0, 5, 'uniform', {'above_bottom': 0}, {'absolute': 23}, 0),
                 ('buried', 3, 1, 13, 'trapezoid', {'above_bottom': 0}, {'absolute': 4}, 0)],
    'lead': [('normal', 8, 0.25, 9, 'trapezoid', {'above_bottom': -24}, {'absolute': 64}, 0)],
}
for m, configs in ORE_GEN.items():
    features = []
    for name, count, discard, size, shape, lo, hi, plateau in configs:
        fid = f'ore_{m}_{name}'
        # 26.3 format: fields sit on the feature itself (no `config`), states are block ids.
        put(f'data/{NS}/worldgen/feature/{fid}.json', {
            'type': 'minecraft:ore',
            'size': size, 'discard_chance_on_air_exposure': discard,
            'targets': [
                {'target': {'predicate_type': 'minecraft:tag_match', 'tag': 'minecraft:stone_ore_replaceables'},
                 'state': f'{NS}:{m}_ore'},
                {'target': {'predicate_type': 'minecraft:tag_match',
                            'tag': 'minecraft:deepslate_ore_replaceables'},
                 'state': f'{NS}:deepslate_{m}_ore'},
            ],
        })
        height = {'type': f'minecraft:{shape}', 'min_inclusive': lo, 'max_inclusive': hi}
        if plateau:
            height['plateau'] = plateau
        put(f'data/{NS}/worldgen/placed_feature/{fid}.json', {
            'feature': f'{NS}:{fid}',
            'placement': [{'type': 'minecraft:count', 'count': count}, {'type': 'minecraft:in_square'},
                          {'type': 'minecraft:height_range', 'height': height}, {'type': 'minecraft:biome'}],
        })
        features.append(f'{NS}:{fid}')
    put(f'data/{NS}/neoforge/biome_modifier/{m}.json', {
        'type': 'neoforge:add_features', 'biomes': f'#{NS}:spawn_ores', 'features': features,
        'step': 'underground_ores'})

# Salt disks (WorldConfig: 2 per chunk, radius 2-3, half height 1), as vanilla's clay disks.
put(f'data/{NS}/worldgen/feature/salt.json', {
    'type': 'minecraft:disk',
    'state_provider': {'fallback': {'type': 'minecraft:simple_state_provider', 'state': f'{NS}:block_salt'},
                       'rules': []},
    'target': {'type': 'minecraft:matching_blocks', 'blocks': ['minecraft:dirt', 'minecraft:clay']},
    'radius': {'type': 'minecraft:uniform', 'min_inclusive': 2, 'max_inclusive': 3},
    'half_height': 1,
})
put(f'data/{NS}/worldgen/placed_feature/salt.json', {
    'feature': f'{NS}:salt',
    'placement': [{'type': 'minecraft:count', 'count': 2}, {'type': 'minecraft:in_square'},
                  {'type': 'minecraft:heightmap', 'heightmap': 'OCEAN_FLOOR_WG'},
                  {'type': 'minecraft:block_predicate_filter',
                   'predicate': {'type': 'minecraft:matching_fluids', 'fluids': 'minecraft:water'}},
                  {'type': 'minecraft:biome'}],
})
put(f'data/{NS}/neoforge/biome_modifier/salt.json', {
    'type': 'neoforge:add_features', 'biomes': f'#{NS}:spawn_ores', 'features': f'{NS}:salt',
    'step': 'underground_ores'})

# Biomes that get Mekanism ores and salt (upstream tag `mekanism:spawn_ores`).
put(f'data/{NS}/tags/worldgen/biome/spawn_ores.json',
    {'replace': False, 'values': ['#minecraft:is_overworld', {'id': '#c:is_overworld', 'required': False}]})

# NeoForge's conventional item tags for vanilla items that imported recipes name. Horizon has no
# NeoForge, so nothing else defines them (members as NeoForge's `Tags.Items`).
COLORS = ['white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray',
          'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black']
CONVENTION_TAGS = {
    'cobblestones/normal': ['minecraft:cobblestone'],
    'glass_blocks/cheap': ['minecraft:glass'] + [f'minecraft:{c}_stained_glass' for c in COLORS],
    'glass_panes': ['minecraft:glass_pane'] + [f'minecraft:{c}_stained_glass_pane' for c in COLORS],
    'ingots/copper': ['minecraft:copper_ingot'],
    'mushrooms': ['minecraft:brown_mushroom', 'minecraft:red_mushroom'],
    'netherracks': ['minecraft:netherrack'],
    'player_workstations/furnaces': ['minecraft:furnace'],
    'rods/wooden': ['minecraft:stick'],
    'sands': ['minecraft:sand', 'minecraft:red_sand'],
    'storage_blocks/coal': ['minecraft:coal_block'],
    'storage_blocks/redstone': ['minecraft:redstone_block'],
}
for tag, values in CONVENTION_TAGS.items():
    put(f'data/c/tags/item/{tag}.json', {'replace': False, 'values': values})

# Base tag for hand-written blocks; import_upstream.py merges Mekanism's own tags into it.
put('data/minecraft/tags/block/mineable/pickaxe.json',
    {'replace': False, 'values': [f'{NS}:{t}_energy_cube' for t in CUBE_TIERS] + [f'{NS}:metallurgic_infuser']})
put('data/minecraft/tags/block/mineable/pickaxe.json',
    {'replace': False, 'values': [f'{GEN}:heat_generator', f'{GEN}:solar_generator']}, GEN)

# Hand-written content stays declared too, so the importer covers it.
for ns, reg in registry.items():
    for kind in ('item', 'block'):
        for f in (ROOT / 'content' / ns / f'data/{ns}/horizon/{kind}').glob('*.json'):
            if f.stem not in reg[f'{kind}s']:
                reg[f'{kind}s'].append(f.stem)
REGISTRY.write_text(json.dumps({ns: {k: sorted(v) for k, v in reg.items()} for ns, reg in registry.items()},
                               indent=2) + '\n', encoding='utf-8')
for ns, reg in registry.items():
    print(f'{ns}: {len(reg["items"])} items, {len(reg["blocks"])} blocks declared')
