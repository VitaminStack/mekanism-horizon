"""Complete `content/mekanismgenerators` with everything that needs no native code (M12).

Run after `scripts/gen_resources.py`, `scripts/import_upstream.py` and `scripts/import_generator_models.py`;
it only adds files (and merges tag/lang entries), it never removes any.

- Plain blocks without multiblock logic (the same as the casings already are): Turbine Valve, Turbine Vent,
  Fission Reactor Port and Logic Adapter, Fusion Reactor Controller, Port and Logic Adapter. Each gets its
  Horizon declaration (vanilla template plus Mekanism's strength, `active`/`mode` state properties as in the
  upstream blockstate), blockstate, item definition, models, loot table and recipe (when every ingredient exists).
  They are registered here and not in `scripts/gen_resources.py` (another session's file), so they are not in
  `scripts/registry.json`: the generator step that reads it does not know them.
- Textures upstream ships that are not here yet (reactor/turbine faces, CTM sheets, GUI sprites, module icons).
- Every key of upstream's `en_us.json` / `de_de.json` (names, tags, tooltips, GUI text, config), also for
  what is not ported: the strings cost nothing and are in place once the feature arrives.
- Members of upstream tags (own, `minecraft`, `c` namespace; block and item) that name the new blocks.

Not ported (needs native code or a Horizon feature that does not exist): the multiblocks themselves, the
fluids and buckets (Horizon knows only vanilla fluids), chemicals, the modules, Superheating Element (no
registry entry upstream), the Wind Generator blades model (OBJ loader). See docs/GENERATORS.md and
docs/sessions/m12-content-parity.md.

Mekanism is MIT licensed (content/mekanismgenerators/LICENSE-MEKANISM).
Usage: python scripts/import_generator_extras.py <path to Mekanism checkout, branch 26.3>
"""
from pathlib import Path
import json
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
NS = 'mekanismgenerators'
OUT = ROOT / 'content' / NS
ACTIVE = {'active': {'type': 'boolean'}}
# id -> (state_properties, default_state). Models in the blockstates carry the rest.
BLOCKS = {
    'turbine_valve': ({}, {}),
    'turbine_vent': ({}, {}),
    'fission_reactor_port': ({'mode': {'type': 'enum', 'values': ['input', 'output_coolant', 'output_waste']}},
                             {'mode': 'input'}),
    'fission_reactor_logic_adapter': ({}, {}),
    'fusion_reactor_controller': (ACTIVE, {'active': 'false'}),
    'fusion_reactor_port': (ACTIVE, {'active': 'false'}),
    'fusion_reactor_logic_adapter': ({}, {}),
}
# Upstream recipe ids (under data/mekanismgenerators/recipe) of the new blocks.
RECIPES = ['turbine/valve', 'turbine/vent', 'fission_reactor/port', 'fission_reactor/logic_adapter',
           'reactor/controller', 'reactor/port', 'reactor/logic_adapter']


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def dump(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def copy(src, dst, overwrite=False):
    if dst.exists() and not overwrite:
        return False
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst)
    return True


def model_refs(obj):
    """Own-namespace models and textures a model/blockstate names (recursively)."""
    found = set()
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k == 'model' and isinstance(v, str):
                found.add(v)
            elif k == 'parent' and isinstance(v, str):
                found.add(v)
            else:
                found |= model_refs(v)
    elif isinstance(obj, list):
        for v in obj:
            found |= model_refs(v)
    return found


def main(up):
    gen = up / 'src/datagen/generated' / NS
    res = up / 'src/generators/resources/assets' / NS
    reg = load(ROOT / 'scripts/registry.json')
    known = {f'{ns}:{i}' for ns, r in reg.items() for i in r['items'] + r['blocks']}
    known |= {f'{NS}:{p.stem}' for kind in ('block', 'item') for p in (OUT / f'data/{NS}/horizon/{kind}').glob('*.json')}
    known |= {f'{NS}:{i}' for i in BLOCKS}

    # --- blocks ---
    for id_, (props, default) in BLOCKS.items():
        decl = {
            'template': 'minecraft:iron_block',
            'properties': {'hardness': 3.5, 'explosion_resistance': 16.0, 'requires_tool': True,
                           'sounds': 'minecraft:stone'},
            'item': True,
        }
        if props:
            decl['state_properties'] = props
            decl['default_state'] = default
        dump(OUT / f'data/{NS}/horizon/block/{id_}.json', decl)
        copy(gen / f'data/{NS}/loot_table/blocks/{id_}.json', OUT / f'data/{NS}/loot_table/blocks/{id_}.json', True)
        copy(res / f'blockstates/{id_}.json', OUT / f'assets/{NS}/blockstates/{id_}.json', True)
        copy(gen / f'assets/{NS}/items/{id_}.json', OUT / f'assets/{NS}/items/{id_}.json', True)

    # --- models of the new blocks (parents included; existing files are kept) ---
    todo = set()
    for id_ in BLOCKS:
        for sub in ('blockstates', 'items'):
            todo |= {r for r in model_refs(load(OUT / f'assets/{NS}/{sub}/{id_}.json')) if r.startswith(f'{NS}:')}
    done = set()
    while todo:
        ref = todo.pop()
        done.add(ref)
        path = ref.split(':', 1)[1]
        src = res / f'models/{path}.json'
        if not src.is_file():
            src = gen / f'assets/{NS}/models/{path}.json'
        copy(src, OUT / f'assets/{NS}/models/{path}.json')
        todo |= {r for r in model_refs(load(src)) if r.startswith(f'{NS}:')} - done

    # --- textures: everything upstream ships and is not here yet ---
    n_tex = 0
    for src in (res / 'textures').rglob('*'):
        if src.is_file() and src.suffix in ('.png', '.mcmeta'):
            n_tex += copy(src, OUT / f'assets/{NS}/textures' / src.relative_to(res / 'textures'))

    # --- recipes of the new blocks, when every Mekanism-family id exists ---
    skipped = []
    for rel in RECIPES:
        recipe = load(gen / f'data/{NS}/recipe/{rel}.json')
        refs = {f'{m.group(2)}:{m.group(3)}' for m in re.finditer(
            r'"(#?)(mekanism|mekanismgenerators):([a-z0-9_/.]+)"',
            json.dumps({k: v for k, v in recipe.items() if k != 'group'})) if not m.group(1)}
        if not refs <= known:
            skipped.append((rel, sorted(refs - known)))
            continue
        copy(gen / f'data/{NS}/recipe/{rel}.json', OUT / f'data/{NS}/recipe/{rel}.json', True)

    # --- tags: merge members that name a new block ---
    new_ids = {f'{NS}:{i}' for i in BLOCKS}
    n_tag = 0
    for d in (NS, 'minecraft', 'c'):
        for kind in ('block', 'item'):
            tag_dir = gen / 'data' / d / 'tags' / kind
            if not tag_dir.is_dir():
                continue
            for src in tag_dir.rglob('*.json'):
                values = [v for v in load(src).get('values', []) if isinstance(v, str) and v in new_ids]
                if not values:
                    continue
                dst = OUT / 'data' / d / 'tags' / kind / src.relative_to(tag_dir)
                old = load(dst)['values'] if dst.exists() else []
                dump(dst, {'replace': False, 'values': list(dict.fromkeys(old + values))})
                n_tag += 1

    # --- names ---
    for lang, src in (('en_us', gen / f'assets/{NS}/lang/en_us.json'), ('de_de', res / 'lang/de_de.json')):
        dst = OUT / f'assets/{NS}/lang/{lang}.json'
        ours = load(dst) if dst.exists() else {}
        ours.update(load(src))
        dump(dst, dict(sorted(ours.items())))

    print(f'blocks {len(BLOCKS)}, models {len(done)}, new texture files {n_tex}, tags merged {n_tag}')
    for rel, missing in skipped:
        print(f'  skipped recipe {rel}: {missing}')


if __name__ == '__main__':
    main(Path(sys.argv[1]))
