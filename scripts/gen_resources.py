"""Generate the Horizon-specific half of Mekanism's resource content (M02):
registry declarations (`data/mekanism/horizon/`), block loot and ore
generation. Assets, recipes, tags and names come from upstream through
`scripts/import_upstream.py`, which runs after this one.

IDs and ore-generation defaults follow Mekanism 26.3 (`MekanismItems`,
`MekanismBlocks`, `OreType`, `WorldConfig`). Run: `python scripts/gen_resources.py`.
"""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[1]
C = ROOT / 'content' / 'mekanism'
NS = 'mekanism'
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

items, blocks = [], []          # registry ids in declaration order


def put(rel, obj):
    path = C / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def item(id_, template='minecraft:iron_ingot'):
    items.append(id_)
    put(f'data/{NS}/horizon/item/{id_}.json', {'template': template, 'rarity': 'common', 'max_stack_size': 64})


def loot(id_, drop, count=None):
    entry = {'type': 'minecraft:item', 'name': f'{NS}:{drop}'}
    if count:
        entry['functions'] = [{'function': 'minecraft:set_count',
                               'count': {'type': 'minecraft:uniform', 'min': count[0], 'max': count[1]}}]
    put(f'data/{NS}/loot_table/blocks/{id_}.json', {
        'type': 'minecraft:block',
        'pools': [{'condition': {'type': 'minecraft:survives_explosion'}, 'entries': [entry], 'rolls': 1}],
        'random_sequence': f'{NS}:blocks/{id_}',
    })


def block(id_, template, hardness, resistance, drop=None, count=None, requires_tool=True):
    blocks.append(id_)
    put(f'data/{NS}/horizon/block/{id_}.json', {
        'template': template,
        'properties': {'hardness': hardness, 'explosion_resistance': resistance, 'requires_tool': requires_tool},
        'item': True,
    })
    loot(id_, drop or id_, count)


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
for m in PRIMARY:
    block(f'block_{m}', 'minecraft:iron_block', 5.0, 6.0)
    block(f'block_raw_{m}', 'minecraft:raw_iron_block', 5.0, 6.0)
    block(f'{m}_ore', 'minecraft:iron_ore', 3.0, 3.0, f'raw_{m}')
    block(f'deepslate_{m}_ore', 'minecraft:deepslate_iron_ore', 4.5, 3.0, f'raw_{m}')
for a in ALLOYS:
    # Refined obsidian is blast-proof in Mekanism (resistance 1200).
    block(f'block_{a}', 'minecraft:iron_block', 50.0 if a == 'refined_obsidian' else 5.0,
          1200.0 if a == 'refined_obsidian' else 6.0)
block('block_fluorite', 'minecraft:quartz_block', 5.0, 6.0)
block('fluorite_ore', 'minecraft:diamond_ore', 3.0, 3.0, 'fluorite_gem', (1, 4))
block('deepslate_fluorite_ore', 'minecraft:deepslate_diamond_ore', 4.5, 3.0, 'fluorite_gem', (1, 4))
block('block_charcoal', 'minecraft:coal_block', 5.0, 6.0)
block('block_salt', 'minecraft:clay', 0.5, 0.5, 'salt', (4, 4), requires_tool=False)
block('steel_casing', 'minecraft:iron_block', 3.5, 9.0)

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

# Base tag for hand-written blocks; import_upstream.py merges Mekanism's own tags into it.
put('data/minecraft/tags/block/mineable/pickaxe.json', {'replace': False, 'values': [f'{NS}:basic_energy_cube']})

# Hand-written content stays declared too, so the importer covers it.
for f in (C / f'data/{NS}/horizon/item').glob('*.json'):
    if f.stem not in items:
        items.append(f.stem)
for f in (C / f'data/{NS}/horizon/block').glob('*.json'):
    if f.stem not in blocks:
        blocks.append(f.stem)
REGISTRY.write_text(json.dumps({'items': sorted(items), 'blocks': sorted(blocks)}, indent=2) + '\n',
                    encoding='utf-8')
print(f'{len(items)} items, {len(blocks)} blocks declared')
