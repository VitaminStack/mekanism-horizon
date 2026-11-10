"""Generate `content/mekanismadditions` (M12, Mekanism: Additions): plastic blocks, glow panels, Obsidian TNT.

Horizon block declarations (`data/mekanismadditions/horizon/block/`) are built here from Mekanism 26.3's
`AdditionsBlocks`; each block is a vanilla template (stairs, slab, fence, fence gate, glass, TNT, ...) plus
properties, so no native code is needed. Assets, loot tables, recipes, tags and names come from a Mekanism
26.3 checkout (sparse clone of `src/additions`, `src/datagen/generated/mekanismadditions`, `src/api/java/mekanism/api/text`).

Upstream tints the grey plastic textures per colour at render time (block/item colour handlers). Horizon
content has no block colour provider, so every colour gets its own pre-tinted copy of each texture and its
own models; the shared `colored/*` models are copied without `tintindex`.

Usage: python scripts/gen_additions.py <path to Mekanism checkout>
"""
from pathlib import Path
import json
import re
import shutil
import sys

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
NS = 'mekanismadditions'
OUT = ROOT / 'content' / NS

# Vanilla MapColor ids (0..61) used by upstream's EnumColor.
MAP = {'white': 8, 'orange': 15, 'magenta': 16, 'light_blue': 17, 'yellow': 18, 'lime': 19, 'pink': 20,
       'gray': 21, 'light_gray': 22, 'cyan': 23, 'purple': 24, 'blue': 25, 'brown': 26, 'green': 27,
       'red': 28, 'black': 29, 'dark_red': 35, 'aqua': 17}
STONE = 'minecraft:stone'
# Textures that upstream tints (plain file name under textures/block).
TINTED = ['plastic_block', 'slick_plastic_block', 'plastic_glow_block', 'reinforced_plastic_block',
          'plastic_road', 'transparent_plastic_block', 'plastic_fence', 'glow_panel']

# suffix -> (template, extra properties). `solid` kinds share hardness 5 / resistance 6 (AdditionsBlocks).
SOLID = {'hardness': 5.0, 'explosion_resistance': 6.0}
KINDS = {
    '_plastic': ('minecraft:stone', SOLID),
    '_slick_plastic': ('minecraft:stone', {**SOLID, 'friction': 0.98}),
    '_plastic_glow': ('minecraft:stone', {**SOLID, 'light': 10}),
    '_reinforced_plastic': ('minecraft:stone', {'hardness': 50.0, 'explosion_resistance': 1200.0}),
    '_plastic_road': ('minecraft:stone', SOLID),
    '_plastic_transparent': ('minecraft:glass', {**SOLID, 'occludes': False}),
    '_plastic_stairs': ('minecraft:stone_stairs', SOLID),
    '_plastic_slab': ('minecraft:stone_slab', SOLID),
    '_plastic_fence': ('minecraft:nether_brick_fence', SOLID),
    '_plastic_fence_gate': ('minecraft:oak_fence_gate', SOLID),
    '_plastic_glow_stairs': ('minecraft:stone_stairs', {**SOLID, 'light': 10}),
    '_plastic_glow_slab': ('minecraft:stone_slab', {**SOLID, 'light': 10}),
    '_plastic_transparent_stairs': ('minecraft:stone_stairs', {**SOLID, 'occludes': False}),
    '_plastic_transparent_slab': ('minecraft:stone_slab', {**SOLID, 'occludes': False}),
    '_glow_panel': ('minecraft:small_amethyst_bud', {'hardness': 1.0, 'explosion_resistance': 6.0, 'light': 15}),
}
# BlockShapes.GLOW_PANEL (rotateAllInitialDown): a base plate and a smaller glow plate, on the side opposite `facing`.
PANEL = {
    'down': [[4, 14, 4, 12, 16, 12], [5, 13.5, 5, 11, 14, 11]],
    'up': [[4, 0, 4, 12, 2, 12], [5, 2, 5, 11, 2.5, 11]],
    'north': [[4, 4, 14, 12, 12, 16], [5, 5, 13.5, 11, 11, 14]],
    'south': [[4, 4, 0, 12, 12, 2], [5, 5, 2, 11, 11, 2.5]],
    'east': [[0, 4, 4, 2, 12, 12], [2, 5, 5, 2.5, 11, 11]],
    'west': [[14, 4, 4, 16, 12, 12], [13.5, 5, 5, 14, 11, 11]],
}


def dump(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def strip_tint(obj):
    """Remove `tintindex` from every model face (tints are baked into the textures)."""
    if isinstance(obj, dict):
        return {k: strip_tint(v) for k, v in obj.items() if k != 'tintindex'}
    if isinstance(obj, list):
        return [strip_tint(v) for v in obj]
    return obj


def tint_png(src, dst, rgb):
    img = Image.open(src).convert('RGBA')
    px = [(r * rgb[0] // 255, g * rgb[1] // 255, b * rgb[2] // 255, a) for r, g, b, a in img.getdata()]
    img.putdata(px)
    dst.parent.mkdir(parents=True, exist_ok=True)
    img.save(dst)


def main(up):
    gen = up / 'src/datagen/generated/mekanismadditions'
    res = up / 'src/additions/resources/assets/mekanismadditions'
    if OUT.exists():
        shutil.rmtree(OUT)
    reg = load(ROOT / 'scripts/registry.json')
    known = {f'{ns}:{i}' for ns, r in reg.items() for i in r['items'] + r['blocks']}

    colors = sorted(f.name[:-len('_glow_panel.json')] for f in
                    (gen / 'data/mekanismadditions/loot_table/blocks').glob('*_glow_panel.json'))
    assert len(colors) == 18, colors
    tint = {}
    for c in colors:
        v = load(gen / f'assets/mekanismadditions/items/{c}_glow_panel.json')['model']['tints'][0]['value']
        tint[c] = ((v >> 16) & 255, (v >> 8) & 255, v & 255)

    ids = []

    def block(id_, template, props, extra=None):
        ids.append(id_)
        props = {'requires_tool': False, **props, 'sounds': STONE}
        dump(OUT / f'data/{NS}/horizon/block/{id_}.json',
             {'template': template, 'properties': props, **(extra or {}), 'item': True})

    for c in colors:
        for suffix, (template, props) in KINDS.items():
            props = {**props, 'map_color': MAP[c]}
            extra = None
            if suffix == '_glow_panel':
                shapes = {'property': 'facing', 'values': PANEL, 'default': PANEL['up']}
                extra = {'shapes': shapes, 'collision_shapes': shapes}
            block(c + suffix, template, props, extra)
    # BlockObsidianTNT: vanilla TNT behaviour (see docs/ADDITIONS.md for the missing blast radius).
    block('obsidian_tnt', 'minecraft:tnt', {'hardness': 0.0, 'explosion_resistance': 0.0, 'map_color': 29})
    ids_set = {f'{NS}:{i}' for i in ids}

    # --- loot tables (copied verbatim) ---
    for id_ in ids:
        src = gen / f'data/mekanismadditions/loot_table/blocks/{id_}.json'
        dst = OUT / f'data/{NS}/loot_table/blocks/{id_}.json'
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)

    # --- textures: one tinted copy per colour; models follow ---
    tex_root = res / 'textures/block'
    for c in colors:
        for t in TINTED:
            tint_png(tex_root / f'{t}.png', OUT / f'assets/{NS}/textures/block/{c}/{t}.png', tint[c])
    for rel in ('glow_panel.png', 'models/obsidian_tnt.png'):
        dst = OUT / f'assets/{NS}/textures/block/{rel}'
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(tex_root / rel, dst)

    models = res / 'models/block'
    # shared tint-free shape models and the cube parent
    for f in (models / 'colored').glob('*.json'):
        dump(OUT / f'assets/{NS}/models/block/colored/{f.name}', strip_tint(load(f)))
    dump(OUT / f'assets/{NS}/models/block/colored_cube.json', {'parent': 'minecraft:block/cube_all'})
    shutil.copyfile(models / 'obsidian_tnt.json', dst := OUT / f'assets/{NS}/models/block/obsidian_tnt.json')

    made = set()

    def color_model(c, name):
        """Per-colour copy of upstream `block/plastic/<name>` (or `glow_panel`); returns its model id."""
        out_id = f'{NS}:block/{c}/{name}'
        if (c, name) in made:
            return out_id
        made.add((c, name))
        src = models / ('glow_panel.json' if name == 'glow_panel' else f'plastic/{name}.json')
        data = load(src)
        text = json.dumps(data)
        if name == 'glow_panel':
            # Only the centre plate is tinted: it gets its own tinted texture variable.
            for el in data['elements']:
                for face in el['faces'].values():
                    if 'tintindex' in face:
                        face['texture'] = '#tinted'
                        del face['tintindex']
            data['textures']['tinted'] = f'{NS}:block/{c}/glow_panel'
            data['textures']['particle'] = f'{NS}:block/glow_panel'
        else:
            data = json.loads(text.replace('mekanism:block/colored_cube', f'{NS}:block/colored_cube'))
            for k, v in data.get('textures', {}).items():
                m = re.fullmatch(rf'{NS}:block/(.+)', v)
                if m:
                    data['textures'][k] = f'{NS}:block/{c}/{m.group(1)}'
        dump(OUT / f'assets/{NS}/models/block/{c}/{name}.json', data)
        return out_id

    def redirect(c, obj):
        """Point every `model` ref in a blockstate/item definition at the colour's own model; drop tints."""
        if isinstance(obj, dict):
            out = {}
            for k, v in obj.items():
                if k == 'tints':
                    continue
                if k == 'model' and isinstance(v, str):
                    m = re.fullmatch(rf'{NS}:block/(?:plastic/)?([a-z_]+)', v)
                    out[k] = color_model(c, m.group(1)) if m else v
                else:
                    out[k] = redirect(c, v)
            return out
        if isinstance(obj, list):
            return [redirect(c, v) for v in obj]
        return obj

    for id_ in ids:
        c = next((c for c in colors if id_.startswith(c + '_')), None)
        for kind in ('blockstates', 'items'):
            src = gen / f'assets/mekanismadditions/{kind}/{id_}.json'
            data = load(src) if src.is_file() else load(res / f'{kind}/{id_}.json')
            dump(OUT / f'assets/{NS}/{kind}/{id_}.json', redirect(c, data) if c else data)

    # --- recipes: crafting + enriching whose every Mekanism id is registered (HDPE is not yet) ---
    skipped, n = [], 0
    for src in (gen / 'data/mekanismadditions/recipe').rglob('*.json'):
        rel = src.relative_to(gen / 'data/mekanismadditions/recipe')
        if rel.parts[0] in ('balloon', 'pigment_extracting') or rel.name == 'walkie_talkie.json':
            continue
        text = src.read_text(encoding='utf-8')
        recipe = json.loads(text)
        if recipe['type'] not in ('minecraft:crafting_shaped', 'minecraft:crafting_shapeless', 'mekanism:enriching'):
            continue  # mekanism:painting needs the Painting Machine (M06)
        if 'neoforge:' in text:
            continue  # recolour recipes use NeoForge's difference ingredient
        scan = json.dumps({k: v for k, v in recipe.items() if k not in ('type', 'group')})
        refs = {f'{m.group(2)}:{m.group(3)}' for m in re.finditer(r'"(#?)(mekanism|mekanismadditions):([a-z0-9_/.]+)"', scan)
                if not m.group(1)}
        if not refs <= known | ids_set:
            skipped.append((str(rel), sorted(refs - known - ids_set)))
            continue
        dst = OUT / f'data/{NS}/recipe' / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_text(text, encoding='utf-8')
        n += 1

    # --- tags (own, minecraft and c namespaces; only registered ids, skip `remove`-only tags) ---
    tags = 0
    for d in (NS, 'minecraft', 'c'):
        for kind in ('block', 'item'):
            tag_dir = gen / 'data' / d / 'tags' / kind
            if not tag_dir.is_dir():
                continue
            for src in tag_dir.rglob('*.json'):
                values = load(src).get('values', [])
                values = [v for v in values if not isinstance(v, str) or v.startswith('#')
                          or not v.startswith(f'{NS}:') or v in ids_set]
                # drop a Mekanism-namespace item that is not ours (balloons, walkie-talkie, spawn eggs)
                values = [v for v in values if not (isinstance(v, str) and v.startswith('mekanismadditions:')
                                                    and v not in ids_set)]
                if not values:
                    continue
                dump(OUT / f'data/{d}/tags/{kind}' / src.relative_to(tag_dir), {'replace': False, 'values': values})
                tags += 1

    # --- names ---
    for lang, src in (('en_us', gen / 'assets/mekanismadditions/lang/en_us.json'),
                      ('de_de', res / 'lang/de_de.json')):
        up_lang = load(src)
        ours = {}
        for id_ in ids:
            k = f'block.{NS}.{id_}'
            if k in up_lang:
                ours[k] = up_lang[k]
        dump(OUT / f'assets/{NS}/lang/{lang}.json', dict(sorted(ours.items())))

    shutil.copyfile(up / 'LICENSE', OUT / 'LICENSE-MEKANISM')
    (OUT / 'horizon-mod.toml').write_text(
        'id = "mekanismadditions"\nname = "Mekanism Additions Horizon (Prototype)"\nversion = "0.1.0"\n'
        'description = "Unofficial experimental port of Mekanism: Additions (plastic blocks, glow panels, Obsidian TNT)."\n'
        'authors = ["VitaminStack"]\napi = "0.2"\nside = "both"\ncapabilities = ["registries"]\n', encoding='utf-8')
    print(f'blocks {len(ids)}, recipes {n}, tags {tags}, models {len(made)}')
    for rel, missing in skipped[:3]:
        print('  skipped', rel, missing)
    print(f'  skipped recipes: {len(skipped)}')


if __name__ == '__main__':
    main(Path(sys.argv[1]))
