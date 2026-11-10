"""Generate `content/mekanismtools` (M12, Mekanism: Tools).

Horizon item definitions (`data/mekanismtools/horizon/item/`) come from the material tables below,
which hold Mekanism 26.3's defaults (`mekanism.tools.common.material.impl.*MaterialDefaults`,
`ToolsMaterialConfig`, `ToolCollection`, `ArmorCollection`). A Horizon item is a vanilla template plus
`components`; tool stats, armor, shield and spear behaviour are vanilla data components, so no native
code is needed. Assets, recipes, tags and names are copied from a Mekanism 26.3 checkout (sparse clone of
`src/tools` and `src/datagen/generated/mekanismtools` is enough), with the NeoForge-only parts replaced
(see docs/TOOLS.md).

Usage: python scripts/gen_tools.py <path to Mekanism checkout>
"""
from pathlib import Path
import json
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
NS = 'mekanismtools'
OUT = ROOT / 'content' / NS
SHIELD_PARTICLE = 'mekanism:block/block_bronze'

# Vanilla materials that only get a paxel: template axe, durability, mining speed, attack bonus,
# enchantability, incorrect-drops tag, axe damage baseline (ToolsMaterialConfig).
VANILLA = {
    'wood': ('wooden', 59, 2, 0, 15, 'wooden', 6, False),
    'stone': ('stone', 131, 4, 1, 5, 'stone', 7, False),
    'copper': ('copper', 190, 5, 1, 13, 'copper', 7, False),
    'iron': ('iron', 250, 6, 2, 14, 'iron', 6, False),
    'gold': ('golden', 32, 12, 0, 22, 'gold', 6, False),
    'diamond': ('diamond', 1561, 8, 3, 10, 'diamond', 5, False),
    'netherite': ('netherite', 2031, 9, 4, 15, 'netherite', 5, True),
}

# Mekanism's own materials. Armor lists are (boots, leggings, chestplate, helmet); horse/nautilus armor
# (ArmorType.BODY) has its own defense and no durability.
MATERIALS = {
    'bronze': dict(
        repair='#c:ingots/bronze', axe=(7, -3.0), durability=375, speed=7, bonus=2, ench=10, armor_ench=5,
        shield=(403, 0.2, 1, 1, 2.5), toughness=1, knockback=0, equip='iron',
        a_dur=(234, 270, 288, 198), a_def=(2, 6, 7, 3), body=6,
        spear=(0.9, 0.88, 0.65, 3.25, 11.5, 7.5, 11.75)),
    'lapis_lazuli': dict(
        repair='#c:gems/lapis', axe=(4, -2.9), durability=128, speed=9, bonus=1, ench=32, armor_ench=35,
        shield=(224, 0.1, 0.8, 1, 1), toughness=0, knockback=0, equip='diamond',
        a_dur=(130, 150, 160, 110), a_def=(1, 3, 4, 1), body=4,
        spear=(0.8, 0.82, 0.65, 4, 13, 8.75, 13.25)),
    'osmium': dict(
        repair='#c:ingots/osmium', axe=(8, -3.3), durability=1024, speed=4, bonus=4, ench=14, armor_ench=9,
        shield=(672, 0.5, 1, 1, 4.5), toughness=3, knockback=0.1, equip='iron',
        a_dur=(390, 450, 480, 330), a_def=(3, 6, 8, 4), body=11,
        spear=(1.15, 1.2, 0.7, 2.5, 8.25, 5.5, 8.5)),
    'refined_glowstone': dict(
        repair='#c:ingots/refined_glowstone', axe=(6, -2.9), durability=384, speed=15, bonus=2, ench=20,
        armor_ench=23, shield=(381, 0.15, 0.9, 1, 2), toughness=0, knockback=0, equip='iron',
        a_dur=(221, 255, 272, 187), a_def=(3, 6, 8, 3), body=11,
        spear=(0.95, 0.95, 0.6, 3, 12, 7.5, 12.5)),
    'refined_obsidian': dict(
        repair='#c:ingots/refined_obsidian', axe=(7, -2.9), durability=4096, speed=12, bonus=8, ench=18,
        armor_ench=13, shield=(1680, 0.6, 1, 1, 6), toughness=5, knockback=0.2, equip='iron',
        fireproof=True, a_dur=(975, 1125, 1200, 825), a_def=(5, 8, 12, 6), body=25,
        spear=(1.55, 1.7, 0.2, 1.5, 5, 2.75, 3.75)),
    'steel': dict(
        repair='#c:ingots/steel', axe=(7, -3.0), durability=500, speed=8, bonus=3, ench=16, armor_ench=11,
        shield=(448, 0.3, 1, 1, 4), toughness=2, knockback=0, equip='iron',
        a_dur=(260, 300, 320, 220), a_def=(3, 6, 8, 3), body=7,
        spear=(1.05, 1.075, 0.55, 2.75, 10.5, 6.5, 10.5)),
}
ARMOR = [('boots', 'feet'), ('leggings', 'legs'), ('chestplate', 'chest'), ('helmet', 'head')]
FIRE = {'types': '#minecraft:is_fire'}
# Mekanism's PaxelRecipe is a shaped recipe that also averages the inputs' durability; Horizon has no such
# type, so it is plain shaped crafting (the paxel starts undamaged).
RECIPE_TYPES = {'mekanismtools:paxel': 'minecraft:crafting_shaped'}

items = []  # ids in declaration order


def put(rel, obj):
    path = OUT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def item(id_, template, components):
    items.append(id_)
    put(f'data/{NS}/horizon/item/{id_}.json', {'template': template, 'components': components})


def modifiers(damage=None, speed=None, armor=None, slot=None, toughness=0.0, knockback=0.0):
    """`minecraft:attribute_modifiers` list in the format of the template items."""
    out = []
    if damage is not None:
        out += [{'type': 'minecraft:attack_damage', 'amount': float(damage), 'id': 'minecraft:base_attack_damage',
                 'operation': 'add_value', 'slot': 'mainhand'},
                {'type': 'minecraft:attack_speed', 'amount': float(speed), 'id': 'minecraft:base_attack_speed',
                 'operation': 'add_value', 'slot': 'mainhand'}]
    if armor is not None:
        mid = f'minecraft:armor.{slot[0]}'
        out += [{'type': 'minecraft:armor', 'amount': float(armor), 'id': mid, 'operation': 'add_value',
                 'slot': slot[1]},
                {'type': 'minecraft:armor_toughness', 'amount': float(toughness), 'id': mid,
                 'operation': 'add_value', 'slot': slot[1]}]
        if knockback:
            out.append({'type': 'minecraft:knockback_resistance', 'amount': float(knockback), 'id': mid,
                        'operation': 'add_value', 'slot': slot[1]})
    return out


def tool_rule(tag_incorrect, tag_mineable, speed):
    return {'rules': [{'blocks': f'#{tag_incorrect}', 'correct_for_drops': False},
                      {'blocks': f'#{tag_mineable}', 'correct_for_drops': True, 'speed': float(speed)}]}


def ticks(seconds):
    return round(seconds * 20)


def gen_items():
    for name, (tmpl, dur, speed, bonus, ench, incorrect, baseline, fire) in VANILLA.items():
        comp = {
            'minecraft:max_damage': 2 * dur,
            'minecraft:enchantable': {'value': ench},
            'minecraft:tool': tool_rule(f'minecraft:incorrect_for_{incorrect}_tool', 'c:mineable/paxel', speed),
            'minecraft:attribute_modifiers': modifiers(baseline + 1 + bonus, -2.4),
            'minecraft:weapon': {'disable_blocking_for_seconds': 5.0, 'item_damage_per_attack': 2},
        }
        if fire:
            comp['minecraft:damage_resistant'] = FIRE
        item(f'{name}_paxel', f'minecraft:{tmpl}_axe', comp)

    for name, m in MATERIALS.items():
        incorrect = f'{NS}:incorrect_for_{name}_tool'
        repair = {'items': m['repair']}
        axe_dmg, axe_speed = m['axe']
        bonus = m['bonus']

        def tool(kind, mineable, damage, atk_speed, template, **extra):
            comp = {
                'minecraft:max_damage': m['durability'],
                'minecraft:enchantable': {'value': m['ench']},
                'minecraft:repairable': repair,
                'minecraft:attribute_modifiers': modifiers(damage + bonus, atk_speed),
                **extra,
            }
            if mineable:
                comp['minecraft:tool'] = tool_rule(incorrect, mineable, m['speed'])
            if m.get('fireproof'):
                comp['minecraft:damage_resistant'] = FIRE
            item(f'{name}_{kind}', f'minecraft:{template}', comp)

        tool('axe', 'minecraft:mineable/axe', axe_dmg, axe_speed, 'iron_axe')
        tool('hoe', 'minecraft:mineable/hoe', -bonus, bonus - 3.0, 'iron_hoe')
        tool('pickaxe', 'minecraft:mineable/pickaxe', 1, -2.8, 'iron_pickaxe')
        tool('shovel', 'minecraft:mineable/shovel', 1.5, -3.0, 'iron_shovel')
        # The sword's tool component (cobweb etc.) has no material in it: keep the template's.
        tool('sword', None, 3, -2.4, 'iron_sword')
        tool('paxel', 'c:mineable/paxel', axe_dmg + 1, -2.4, 'iron_axe',
             **{'minecraft:max_damage': 2 * m['durability']})

        attack_s, dmg_mult, delay_s, dismount_s, dismount_v, knock_s, damage_s = m['spear']
        # attack speed = 1 / attack duration (seconds) - base speed 4
        comp = {
            'minecraft:max_damage': m['durability'],
            'minecraft:enchantable': {'value': m['ench']},
            'minecraft:repairable': repair,
            'minecraft:attack_animation': {'type': 'stab', 'duration': ticks(attack_s)},
            'minecraft:attribute_modifiers': modifiers(bonus, 1 / attack_s - 4),
            'minecraft:kinetic_weapon': {
                'damage_conditions': {'max_duration_ticks': ticks(damage_s), 'min_relative_speed': 4.6},
                'damage_multiplier': dmg_mult,
                'delay_ticks': ticks(delay_s),
                'dismount_conditions': {'max_duration_ticks': ticks(dismount_s), 'min_speed': float(dismount_v)},
                'forward_movement': 0.38,
                'hit_sound': 'minecraft:item.spear.hit',
                'knockback_conditions': {'max_duration_ticks': ticks(knock_s), 'min_speed': 5.1},
                'sound': 'minecraft:item.spear.use',
            },
        }
        if m.get('fireproof'):
            comp['minecraft:damage_resistant'] = FIRE
        item(f'{name}_spear', 'minecraft:iron_spear', comp)

        s_dur, s_delay, s_factor, s_base, s_threshold = m['shield']
        comp = {
            'minecraft:max_damage': s_dur,
            'minecraft:repairable': repair,
            'minecraft:blocks_attacks': {
                'block_delay_seconds': s_delay,
                'block_sound': 'minecraft:item.shield.block',
                'bypassed_by': '#minecraft:bypasses_shield',
                'disabled_sound': 'minecraft:item.shield.break',
                'damage_reductions': [{'horizontal_blocking_angle': 90.0, 'base': 0.0, 'factor': float(s_factor)}],
                'item_damage': {'base': 1.0, 'factor': 1.0, 'threshold': float(s_threshold)},
            },
        }
        if m.get('fireproof'):
            comp['minecraft:damage_resistant'] = FIRE
        item(f'{name}_shield', 'minecraft:shield', comp)

        for (kind, slot), dur, defense in zip(ARMOR, m['a_dur'], m['a_def']):
            comp = {
                'minecraft:max_damage': dur,
                'minecraft:enchantable': {'value': m['armor_ench']},
                'minecraft:repairable': repair,
                'minecraft:attribute_modifiers': modifiers(armor=defense, slot=(kind, slot),
                                                           toughness=m['toughness'], knockback=m['knockback']),
                'minecraft:equippable': {'asset_id': f'{NS}:{name}', 'equip_sound': f'minecraft:item.armor.equip_{m["equip"]}',
                                         'slot': slot},
            }
            if m.get('fireproof'):
                comp['minecraft:damage_resistant'] = FIRE
            item(f'{name}_{kind}', f'minecraft:iron_{kind}', comp)

        for kind in ('horse', 'nautilus'):
            tmpl_equip = {'horse': {'allowed_entities': '#minecraft:can_wear_horse_armor',
                                    'equip_sound': 'minecraft:entity.horse.armor',
                                    'shearing_sound': 'minecraft:item.horse_armor.unequip'},
                          'nautilus': {'allowed_entities': '#minecraft:can_wear_nautilus_armor',
                                       'equip_on_interact': True,
                                       'equip_sound': 'minecraft:item.armor.equip_nautilus',
                                       'shearing_sound': 'minecraft:item.armor.unequip_nautilus'}}[kind]
            comp = {
                'minecraft:attribute_modifiers': modifiers(armor=m['body'], slot=('body', 'body'),
                                                           toughness=m['toughness'], knockback=m['knockback']),
                'minecraft:equippable': {**tmpl_equip, 'asset_id': f'{NS}:{name}', 'can_be_sheared': True,
                                         'damage_on_hurt': False, 'slot': 'body'},
            }
            if m.get('fireproof'):
                comp['minecraft:damage_resistant'] = FIRE
            item(f'{name}_{kind}_armor', f'minecraft:iron_{kind}_armor', comp)


def copy_json(src, dst):
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst)


def import_upstream(up):
    gen = up / 'src/datagen/generated/mekanismtools'
    res = up / 'src/tools/resources/assets/mekanismtools'
    reg = json.loads((ROOT / 'scripts/registry.json').read_text(encoding='utf-8'))
    known = {f'{ns}:{i}' for ns, r in reg.items() for i in r['items'] + r['blocks']} | {f'{NS}:{i}' for i in items}
    assets = OUT / 'assets' / NS

    # item definitions: NeoForge/Mekanism-only model types are replaced (docs/TOOLS.md)
    def fix(node):
        if isinstance(node, dict):
            if node.get('type') == 'neoforge:trimmed_armor':
                return fix(node['base_model'])
            if node.get('type') == 'mekanismtools:shield':
                return {'type': 'minecraft:shield'}
            return {k: fix(v) for k, v in node.items()}
        if isinstance(node, list):
            return [fix(v) for v in node]
        return node

    for id_ in items:
        src = gen / 'assets' / NS / 'items' / f'{id_}.json'
        put(f'assets/{NS}/items/{id_}.json', fix(json.loads(src.read_text(encoding='utf-8'))))
        model = gen / 'assets' / NS / 'models' / 'item'
        for extra in ('', '_blocking', '_in_hand'):
            m = model / f'{id_}{extra}.json'
            if m.is_file():
                copy_json(m, assets / 'models' / 'item' / m.name)
    # textures the models use (item icons, equipment layers)
    textures = set()
    for m in (assets / 'models' / 'item').glob('*.json'):
        for tex in json.loads(m.read_text(encoding='utf-8')).get('textures', {}).values():
            if tex.startswith(f'{NS}:'):
                textures.add(tex.split(':', 1)[1])
    for tex in sorted(textures):
        copy_json(res / 'textures' / f'{tex}.png', assets / 'textures' / f'{tex}.png')
    for layer in ('horse_body', 'humanoid', 'humanoid_baby', 'humanoid_leggings', 'nautilus_body'):
        for src in (res / 'textures/entity/equipment' / layer).glob('*.png'):
            copy_json(src, assets / 'textures/entity/equipment' / layer / src.name)
    for src in (gen / 'assets' / NS / 'equipment').glob('*.json'):
        eq = json.loads(src.read_text(encoding='utf-8'))
        eq.pop('trim_overrides', None)  # palettes live in Mekanism's own pack
        put(f'assets/{NS}/equipment/{src.name}', eq)

    # recipes
    n = 0
    for src in (gen / 'data' / NS / 'recipe').rglob('*.json'):
        recipe = json.loads(src.read_text(encoding='utf-8'))
        recipe['type'] = RECIPE_TYPES.get(recipe['type'], recipe['type'])
        if not recipe['type'].startswith('minecraft:'):
            print(f'  skipped {recipe["type"]} {src.name}')
            continue
        refs = {m for m in re.findall(r'"((?:mekanism|mekanismtools):[a-z0-9_/]+)"', json.dumps(recipe))}
        if not refs <= known:
            print(f'  skipped {src.name}: unknown {sorted(refs - known)}')
            continue
        put(f'data/{NS}/recipe/{src.relative_to(gen / "data" / NS / "recipe")}', recipe)
        n += 1

    # tags: own, vanilla, convention; only ids that exist
    t = 0
    for ns in (NS, 'minecraft', 'c'):
        for kind in ('block', 'item'):
            tag_dir = gen / 'data' / ns / 'tags' / kind
            if not tag_dir.is_dir():
                continue
            for src in tag_dir.rglob('*.json'):
                values = [v for v in json.loads(src.read_text(encoding='utf-8'))['values']
                          if v.startswith('#') or not v.startswith('mekanism') or v in known]
                put(f'data/{ns}/tags/{kind}/{src.relative_to(tag_dir)}', {'replace': False, 'values': values})
                t += 1

    # names
    for lang in ('en_us', 'de_de'):
        src = gen / 'assets' / NS / 'lang' / f'{lang}.json'
        if not src.is_file():
            src = res / 'lang' / f'{lang}.json'
        up_lang = json.loads(src.read_text(encoding='utf-8'))
        put(f'assets/{NS}/lang/{lang}.json',
            {f'item.{NS}.{i}': up_lang[f'item.{NS}.{i}'] for i in items if f'item.{NS}.{i}' in up_lang})
    shutil.copyfile(up / 'LICENSE', OUT / 'LICENSE-MEKANISM')
    print(f'items {len(items)}, textures {len(textures)}, recipes {n}, tags {t}')


if __name__ == '__main__':
    gen_items()
    import_upstream(Path(sys.argv[1]))
