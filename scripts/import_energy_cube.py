"""Turn Mekanism's energy cube model into plain block models Horizon can draw.

Mekanism draws the cube with its own model loader (`mekanism:energy_cube`): one
model file with a `frame` (edges and tier-coloured corners) and, per relative
side, a `ports` part shown where the side config is not "none". Horizon has no
such loader, so this script writes:

- `models/block/energy_cube/base.json`: the frame as ordinary `elements`;
- `models/block/energy_cube/port.json`: the front port (north face);
- `models/block/<tier>_energy_cube.json`: the frame with the tier's corner texture;
- a multipart blockstate per tier: the frame always, and the port turned onto
  each side whose `port_<side>` state property is true (set by the native mod);
- the item model (frame, Mekanism's display transforms) and item definition;
- the textures these use.

Not ported: the status LEDs (lit by energy level) and the energy core that
Mekanism's block entity renderer draws inside the frame.

Mekanism is MIT licensed (content/mekanism/LICENSE-MEKANISM).
Usage: python scripts/import_energy_cube.py <path to Mekanism checkout, branch 26.3>
"""
from pathlib import Path
import json
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
A = ROOT / 'content' / 'mekanism' / 'assets' / 'mekanism'
NS = 'mekanism'
TIERS = ['basic', 'advanced', 'elite', 'ultimate', 'creative']
# Rotation that turns the north-facing port onto each side (as vanilla's dispenser).
PORT_ROTATION = {'north': {}, 'east': {'y': 90}, 'south': {'y': 180}, 'west': {'y': 270},
                 'up': {'x': 270}, 'down': {'x': 90}}


def put(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def strip(elements):
    """Mekanism's elements carry a `name`; vanilla ignores it, keep files small."""
    return [{k: v for k, v in e.items() if k != 'name'} for e in elements]


def main(up):
    res = up / 'src/main/resources/assets/mekanism'
    base = json.loads((res / 'models/block/energy_cube/base.json').read_text(encoding='utf-8'))
    textures = base['textures']
    put(A / 'models/block/energy_cube/base.json', {
        'parent': 'minecraft:block/block',
        'textures': {k: v for k, v in textures.items() if k in ('1', 'particle')},
        'elements': strip(base['frame']),
    })
    put(A / 'models/block/energy_cube/port.json', {
        'textures': {k: textures[k] for k in ('1', '2', '3')} | {'particle': textures['particle']},
        'elements': strip(base['ports']['front']),
    })
    item_display = json.loads((res / 'models/item/basic_energy_cube.json').read_text(encoding='utf-8'))['display']
    for tier in TIERS:
        cid = f'{tier}_energy_cube'
        corner = json.loads((res / f'models/block/energy_cube/{tier}.json').read_text(encoding='utf-8'))
        put(A / f'models/block/{cid}.json', {
            'parent': f'{NS}:block/energy_cube/base', 'textures': corner['textures']})
        put(A / f'models/item/{cid}.json', {'parent': f'{NS}:block/{cid}', 'display': item_display})
        put(A / f'items/{cid}.json', {'model': {'type': 'minecraft:model', 'model': f'{NS}:item/{cid}'}})
        parts = [{'apply': {'model': f'{NS}:block/{cid}'}}]
        for side, rot in PORT_ROTATION.items():
            parts.append({'when': {f'port_{side}': 'true'},
                          'apply': {'model': f'{NS}:block/energy_cube/port', **rot}})
        put(A / f'blockstates/{cid}.json', {'multipart': parts})
    used = {textures[k] for k in ('1', '2', '3', 'particle')} | {
        json.loads((res / f'models/block/energy_cube/{t}.json').read_text(encoding='utf-8'))['textures']['corner']
        for t in TIERS}
    for tex in sorted(used):
        rel = f'textures/{tex[len(NS) + 1:]}.png'
        (A / rel).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(res / rel, A / rel)
    print(f'energy cube: {len(TIERS)} tiers, {len(used)} textures')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(Path(sys.argv[1]))
