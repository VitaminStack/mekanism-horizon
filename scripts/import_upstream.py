"""Import Mekanism's own assets and vanilla-type data for every id we register.

Reads a local checkout of mekanism/Mekanism (branch 26.3) and copies, for the
ids in `scripts/registry.json` (written by `gen_resources.py`):

- blockstates, item definitions, models (following `parent`/`model` refs) and
  the textures they use (+ `.mcmeta`), from `src/main/resources` and
  `src/datagen/generated`;
- recipes of vanilla types (`minecraft:crafting_*`, `smelting`, `blasting`)
  whose every `mekanism:` id is registered here, and of the Mekanism machine
  types in MACHINE_RECIPES (chemical tags resolved, `neoforge:or` flattened);
  other machine recipe types are skipped until their machines exist;
- block/item tags (any namespace), filtered to registered ids, merged into ours;
- en_us and de_de names for registered ids.

Mekanism is MIT licensed; its LICENSE is copied to content/mekanism/LICENSE-MEKANISM.

Usage: python scripts/import_upstream.py <path to Mekanism checkout>
"""
from pathlib import Path
import json
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
C = ROOT / 'content' / 'mekanism'
NS = 'mekanism'
VANILLA_RECIPES = {'minecraft:crafting_shaped', 'minecraft:crafting_shapeless', 'minecraft:smelting',
                   'minecraft:blasting', 'minecraft:smoking', 'minecraft:campfire_cooking',
                   'minecraft:stonecutting'}
# Assets written elsewhere: the energy cubes by scripts/import_energy_cube.py (custom model loader
# turned into plain models), the debug blocks by gen_resources.py.
ASSET_SKIP = {f'{t}_energy_cube' for t in ('basic', 'advanced', 'elite', 'ultimate', 'creative')} | {
    'debug_energy_source', 'debug_energy_sink'}
REF = re.compile(r'"#?mekanism:([a-z0-9_/.]+)"')
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


def main(up):
    gen = up / 'src/datagen/generated/mekanism'
    main_res = up / 'src/main/resources'
    asset_roots = [gen / 'assets/mekanism', main_res / 'assets/mekanism']
    reg = json.loads((ROOT / 'scripts/registry.json').read_text(encoding='utf-8'))
    known = set(reg['items']) | set(reg['blocks'])
    stats = {}

    def find(rel):
        for r in asset_roots:
            if (r / rel).is_file():
                return r / rel
        return None

    def copy_asset(rel):
        src = find(rel)
        if not src:
            return None
        dst = C / 'assets/mekanism' / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        return src

    # --- models + textures, followed transitively ---
    seen_models, textures = set(), set()

    def model(ref):
        if not ref.startswith(f'{NS}:') or ref in seen_models:
            return
        seen_models.add(ref)
        src = copy_asset(f'models/{ref[len(NS) + 1:]}.json')
        if not src:
            print(f'  missing model {ref}')
            return
        data = json.loads(src.read_text(encoding='utf-8'))
        if 'parent' in data:
            model(data['parent'])
        for tex in data.get('textures', {}).values():
            if isinstance(tex, str) and tex.startswith(f'{NS}:'):
                textures.add(tex[len(NS) + 1:])
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

    for id_ in sorted(known - ASSET_SKIP):
        for kind in ('blockstates', 'items'):
            src = copy_asset(f'{kind}/{id_}.json')
            if src:
                walk_models(json.loads(src.read_text(encoding='utf-8')))
            elif kind == 'items':
                print(f'  no upstream item definition for {id_}')
    for tex in sorted(textures):
        for suffix in ('.png', '.png.mcmeta'):
            if not copy_asset(f'textures/{tex}{suffix}') and suffix == '.png':
                print(f'  missing texture {tex}')
    stats['models'], stats['textures'] = len(seen_models), len(textures)

    # --- recipes ---
    chem_tags = {}
    for f in (gen / 'data/mekanism/tags/mekanism/chemical').glob('*.json'):
        values = json.loads(f.read_text(encoding='utf-8')).get('values', [])
        if len(values) == 1 and isinstance(values[0], str):
            chem_tags[f'#{NS}:{f.stem}'] = values[0]
    n = 0
    for src in (gen / 'data/mekanism/recipe').rglob('*.json'):
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
        # `type` and `group` name no item, chemicals are no items; every other plain `mekanism:` id
        # must be registered.
        skip = ('type', 'group', 'chemical_input') + (('output',) if kind == 'mekanism:chemical_conversion' else ())
        scan = json.dumps({k: v for k, v in recipe.items() if k not in skip})
        refs = {m.group(1) for m in REF.finditer(scan) if not m.group(0).startswith('"#')}
        if refs <= known:
            dst = C / 'data/mekanism/recipe' / src.relative_to(gen / 'data/mekanism/recipe')
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_text(text, encoding='utf-8')
            n += 1
    stats['recipes'] = n

    # --- tags (block + item), filtered and merged ---
    n = 0
    # Horizon lets a content mod add tags only to its own, vanilla and convention (c) namespaces.
    for data_ns in (gen / 'data' / n for n in ('mekanism', 'minecraft', 'c')):
        for kind in ('block', 'item'):
            tag_dir = data_ns / 'tags' / kind
            if not tag_dir.is_dir():
                continue
            for src in tag_dir.rglob('*.json'):
                values = json.loads(src.read_text(encoding='utf-8')).get('values', [])
                keep = [v for v in values if not (isinstance(v, str) and v.startswith(f'{NS}:'))
                        or v[len(NS) + 1:] in known]
                if not keep:
                    continue
                dst = C / 'data' / data_ns.name / 'tags' / kind / src.relative_to(tag_dir)
                old = json.loads(dst.read_text(encoding='utf-8'))['values'] if dst.exists() else []
                dst.parent.mkdir(parents=True, exist_ok=True)
                dst.write_text(json.dumps({'replace': False, 'values': list(dict.fromkeys(old + keep))},
                                          indent=2) + '\n', encoding='utf-8')
                n += 1
    stats['tags'] = n

    # --- names ---
    for lang in ('en_us', 'de_de'):
        src = find(f'lang/{lang}.json')
        if not src:
            print(f'  no upstream {lang}')
            continue
        up_lang = json.loads(src.read_text(encoding='utf-8'))
        dst = C / f'assets/mekanism/lang/{lang}.json'
        ours = json.loads(dst.read_text(encoding='utf-8')) if dst.exists() else {}
        for id_ in known:
            for key in (f'item.{NS}.{id_}', f'block.{NS}.{id_}'):
                if key in up_lang:
                    ours[key] = up_lang[key]
        dst.write_text(json.dumps(dict(sorted(ours.items())), indent=2, ensure_ascii=False) + '\n',
                       encoding='utf-8')

    shutil.copyfile(up / 'LICENSE', C / 'LICENSE-MEKANISM')
    print('imported:', stats)


if __name__ == '__main__':
    main(Path(sys.argv[1]))
