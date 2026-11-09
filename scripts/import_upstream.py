"""Import Mekanism's own assets and vanilla-type data for every id we register.

Reads a local checkout of mekanism/Mekanism (branch 26.3) and copies, for the
ids in `scripts/registry.json` (written by `gen_resources.py`), into the content
mod of each id's namespace (`content/mekanism`, `content/mekanismgenerators`):

- blockstates, item definitions, models (following `parent`/`model` refs, also
  into the other namespace) and the textures they use (+ `.mcmeta`), from
  `src/main/resources`, `src/generators/resources` and `src/datagen/generated`;
- recipes of vanilla types (`minecraft:crafting_*`, `smelting`, `blasting`)
  whose every Mekanism id is registered here, and of the Mekanism machine
  types in MACHINE_RECIPES (chemical tags resolved, `neoforge:or` flattened);
  other machine recipe types are skipped until their machines exist;
- block/item tags (own, vanilla and convention namespaces), filtered to
  registered ids, merged into ours;
- en_us and de_de names for registered ids.

Mekanism is MIT licensed; its LICENSE is copied to each mod's LICENSE-MEKANISM.

Usage: python scripts/import_upstream.py <path to Mekanism checkout>
"""
from pathlib import Path
import json
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
NS = 'mekanism'
GEN = 'mekanismgenerators'
# Upstream source folder of each mod's hand-written resources.
SOURCES = {NS: 'src/main/resources', GEN: 'src/generators/resources'}
VANILLA_RECIPES = {'minecraft:crafting_shaped', 'minecraft:crafting_shapeless', 'minecraft:smelting',
                   'minecraft:blasting', 'minecraft:smoking', 'minecraft:campfire_cooking',
                   'minecraft:stonecutting'}
# Assets written elsewhere: the energy cubes by scripts/import_energy_cube.py (custom model loader
# turned into plain models), the debug blocks by gen_resources.py.
ASSET_SKIP = {f'{NS}:{t}_energy_cube' for t in ('basic', 'advanced', 'elite', 'ultimate', 'creative')} | {
    f'{NS}:debug_energy_source', f'{NS}:debug_energy_sink'}
REF = re.compile(r'"(#?)(mekanism|mekanismgenerators):([a-z0-9_/.]+)"')
# Mekanism machine recipe types whose machine exists here.
MACHINE_RECIPES = {'mekanism:metallurgic_infusing', 'mekanism:chemical_conversion'}
# Chemicals with Mekanism's infuse-type attribute: what the Metallurgic Infuser takes.
INFUSE_TYPES = {f'{NS}:{t}' for t in ('carbon', 'redstone', 'diamond', 'refined_obsidian', 'tin', 'gold',
                                      'bio', 'fungi')}


def flat_or(ingredient):
    """NeoForge's `neoforge:or` (any child matches) as a plain list, which Horizon reads."""
    if isinstance(ingredient, dict) and ingredient.get('type') == 'neoforge:or':
        children = ingredient['values']
    elif isinstance(ingredient, list):
        children = ingredient
    else:
        return ingredient
    out = []
    for v in children:
        v = flat_or(v)
        out.extend(v if isinstance(v, list) else [v])
    return out


def content(ns):
    return ROOT / 'content' / ns


def main(up):
    gen = {ns: up / 'src/datagen/generated' / ns for ns in SOURCES}
    asset_roots = {ns: [gen[ns] / 'assets' / ns, up / src / 'assets' / ns] for ns, src in SOURCES.items()}
    reg = json.loads((ROOT / 'scripts/registry.json').read_text(encoding='utf-8'))
    known = {f'{ns}:{i}' for ns, r in reg.items() for i in r['items'] + r['blocks']}
    stats = {}

    def find(ns, rel):
        for r in asset_roots[ns]:
            if (r / rel).is_file():
                return r / rel
        return None

    def copy_asset(ns, rel):
        src = find(ns, rel)
        if not src:
            return None
        dst = content(ns) / 'assets' / ns / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        return src

    def split(ref):
        ns, _, path = ref.partition(':')
        return (ns, path) if ns in SOURCES else (None, None)

    # --- models + textures, followed transitively ---
    seen_models, textures = set(), set()

    def model(ref):
        ns, path = split(ref)
        if not ns or ref in seen_models:
            return
        seen_models.add(ref)
        src = copy_asset(ns, f'models/{path}.json')
        if not src:
            print(f'  missing model {ref}')
            return
        data = json.loads(src.read_text(encoding='utf-8'))
        if 'parent' in data:
            model(data['parent'])
        for tex in data.get('textures', {}).values():
            if isinstance(tex, str) and split(tex)[0]:
                textures.add(tex)
        for v in data.get('children', {}).values() if isinstance(data.get('children'), dict) else []:
            if isinstance(v, dict) and 'parent' in v:
                model(v['parent'])

    def walk_models(obj):
        if isinstance(obj, dict):
            for k, v in obj.items():
                if k == 'model' and isinstance(v, str):
                    model(v)
                else:
                    walk_models(v)
        elif isinstance(obj, list):
            for v in obj:
                walk_models(v)

    for full in sorted(known - ASSET_SKIP):
        ns, id_ = split(full)
        for kind in ('blockstates', 'items'):
            src = copy_asset(ns, f'{kind}/{id_}.json')
            if src:
                walk_models(json.loads(src.read_text(encoding='utf-8')))
            elif kind == 'items':
                print(f'  no upstream item definition for {full}')
    for tex in sorted(textures):
        ns, path = split(tex)
        for suffix in ('.png', '.png.mcmeta'):
            if not copy_asset(ns, f'textures/{path}{suffix}') and suffix == '.png':
                print(f'  missing texture {tex}')
    stats['models'], stats['textures'] = len(seen_models), len(textures)

    # --- recipes ---
    chem_tags = {}
    for f in (gen[NS] / 'data/mekanism/tags/mekanism/chemical').glob('*.json'):
        values = json.loads(f.read_text(encoding='utf-8')).get('values', [])
        if len(values) == 1 and isinstance(values[0], str):
            chem_tags[f'#{NS}:{f.stem}'] = values[0]
    n = 0
    for ns in SOURCES:
        recipe_dir = gen[ns] / 'data' / ns / 'recipe'
        for src in recipe_dir.rglob('*.json'):
            text = src.read_text(encoding='utf-8')
            recipe = json.loads(text)
            kind = recipe.get('type')
            if kind == 'mekanism:mek_data':
                # Shaped crafting that also carries the inputs' stored contents over. Horizon has no such
                # type yet, so it is plain shaped crafting: a crafted cube starts empty.
                recipe['type'] = 'minecraft:crafting_shaped'
                text = json.dumps(recipe, indent=2)
            elif kind in MACHINE_RECIPES:
                # Read by native/mekanism_native (Horizon keeps mod recipe types as their JSON).
                # Infusion conversions only for infuse types; the others feed later machines.
                if kind == 'mekanism:chemical_conversion' and recipe['output']['id'] not in INFUSE_TYPES:
                    continue
                if 'chemical_input' in recipe:
                    tag = recipe['chemical_input']['ingredient']
                    if tag not in chem_tags:
                        print(f'  unresolved chemical {tag} in {src.name}')
                        continue
                    recipe['chemical_input']['ingredient'] = chem_tags[tag]
                for slot in ('input', 'item_input'):
                    if slot in recipe:
                        recipe[slot]['ingredient'] = flat_or(recipe[slot]['ingredient'])
                text = json.dumps(recipe, indent=2)
            elif kind not in VANILLA_RECIPES:
                continue
            # `type` and `group` name no item, chemicals are no items; every other plain Mekanism id
            # must be registered.
            skip = ('type', 'group', 'chemical_input') + (
                ('output',) if kind == 'mekanism:chemical_conversion' else ())
            scan = json.dumps({k: v for k, v in recipe.items() if k not in skip})
            refs = {f'{m.group(2)}:{m.group(3)}' for m in REF.finditer(scan) if not m.group(1)}
            if refs <= known:
                dst = content(ns) / 'data' / ns / 'recipe' / src.relative_to(recipe_dir)
                dst.parent.mkdir(parents=True, exist_ok=True)
                dst.write_text(text, encoding='utf-8')
                n += 1
    stats['recipes'] = n

    # --- tags (block + item), filtered and merged ---
    def keep_value(v):
        return not isinstance(v, str) or v.startswith('#') or not split(v)[0] or v in known

    n = 0
    for ns in SOURCES:
        # Horizon lets a content mod add tags only to its own, vanilla and convention (c) namespaces.
        for data_ns in (gen[ns] / 'data' / d for d in (ns, 'minecraft', 'c')):
            for kind in ('block', 'item'):
                tag_dir = data_ns / 'tags' / kind
                if not tag_dir.is_dir():
                    continue
                for src in tag_dir.rglob('*.json'):
                    values = json.loads(src.read_text(encoding='utf-8')).get('values', [])
                    keep = [v for v in values if keep_value(v)]
                    if not keep:
                        continue
                    dst = content(ns) / 'data' / data_ns.name / 'tags' / kind / src.relative_to(tag_dir)
                    old = json.loads(dst.read_text(encoding='utf-8'))['values'] if dst.exists() else []
                    dst.parent.mkdir(parents=True, exist_ok=True)
                    dst.write_text(json.dumps({'replace': False, 'values': list(dict.fromkeys(old + keep))},
                                              indent=2) + '\n', encoding='utf-8')
                    n += 1
    # A tag naming a Mekanism tag that is not shipped here (the fission reactor's) would fail to load:
    # drop such references, and tags left empty, until none is left.
    tag_files = {f for ns in SOURCES for f in (content(ns) / 'data').glob('*/tags/*/**/*.json')}
    changed = True
    while changed:
        changed = False
        for f in sorted(tag_files):
            kind_dir = f.parent
            while kind_dir.parent.name != 'tags':
                kind_dir = kind_dir.parent

            def exists(ref):
                ns, path = split(ref[1:])
                return not ns or any((content(m) / 'data' / ns / 'tags' / kind_dir.name / f'{path}.json').is_file()
                                     for m in SOURCES)

            data = json.loads(f.read_text(encoding='utf-8'))
            keep = [v for v in data['values'] if not (isinstance(v, str) and v.startswith('#')) or exists(v)]
            if keep == data['values']:
                continue
            changed = True
            if keep:
                f.write_text(json.dumps({**data, 'values': keep}, indent=2) + '\n', encoding='utf-8')
            else:
                f.unlink()
                tag_files.discard(f)
                n -= 1
    stats['tags'] = n

    # --- names ---
    for ns in SOURCES:
        for lang in ('en_us', 'de_de'):
            src = find(ns, f'lang/{lang}.json')
            if not src:
                print(f'  no upstream {ns} {lang}')
                continue
            up_lang = json.loads(src.read_text(encoding='utf-8'))
            dst = content(ns) / f'assets/{ns}/lang/{lang}.json'
            ours = json.loads(dst.read_text(encoding='utf-8')) if dst.exists() else {}
            for id_ in reg[ns]['items'] + reg[ns]['blocks']:
                for key in (f'item.{ns}.{id_}', f'block.{ns}.{id_}'):
                    if key in up_lang:
                        ours[key] = up_lang[key]
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_text(json.dumps(dict(sorted(ours.items())), indent=2, ensure_ascii=False) + '\n',
                           encoding='utf-8')

    for ns in SOURCES:
        shutil.copyfile(up / 'LICENSE', content(ns) / 'LICENSE-MEKANISM')
    print('imported:', stats)


if __name__ == '__main__':
    main(Path(sys.argv[1]))
