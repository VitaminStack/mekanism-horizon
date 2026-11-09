"""Turn Mekanism's Wind Generator model into plain block models Horizon can draw.

Mekanism draws the Wind Generator with NeoForge's composite and OBJ loaders
(`models/block/wind_generator.json`: a `base` of ordinary elements and the
tower as `wind_generator.obj`) and turns the blades with its block entity
renderer. Horizon reads neither loader, and a model's elements must stay
within -16 to 32. The Blockbench sources next to the OBJ files
(`wind_generator.bbmodel`, `java_block` format) hold the same geometry as
ordinary cubes, so this script writes:

- `models/block/wind_generator.json`: the base, the block's own model;
- `models/block/wind_generator/tower_<i>.json` and `rotor_<i>.json`: the
  tower and the blades, each cube moved by whole blocks so it is within range
  (the native mod draws each part moved back, and turns the rotor about its
  hub; `scripts/wind_parts.json` lists the parts, how far each was moved per
  axis, and the hub);
- the textures these use.

Mekanism is MIT licensed (content/mekanismgenerators/LICENSE-MEKANISM).
Usage: python scripts/import_wind_generator.py <path to Mekanism checkout, branch 26.3>
"""
from pathlib import Path
import json
import math
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
NS = 'mekanismgenerators'
A = ROOT / 'content' / NS / 'assets' / NS
# Blockbench texture index to the model's texture variables (as in upstream's `base`).
TEXTURES = ['#wind_generator', '#ports_led', '#ports']
TEXTURE_IDS = {
    'particle': f'{NS}:block/model/wind_generator',
    'wind_generator': f'{NS}:block/model/wind_generator',
    'ports_led': 'mekanism:block/models/ports_led',
    'ports': 'mekanism:block/models/ports',
}
BLADES = ('blade', 'blades_cap')


def element(cube, t):
    """A Blockbench cube as a model element, moved by `t` pixels (its rotation
    origin too, which moves the turned cube by the same)."""
    out = {
        'name': cube['name'],
        'from': [a + b for a, b in zip(cube['from'], t)],
        'to': [a + b for a, b in zip(cube['to'], t)],
    }
    rot = cube.get('rotation')
    if rot and any(rot):
        out['rotation'] = {'x': rot[0], 'y': rot[1], 'z': rot[2],
                           'origin': [a + b for a, b in zip(cube.get('origin', [0, 0, 0]), t)]}
    faces = {}
    for name, face in cube['faces'].items():
        if face.get('texture') is None:
            continue  # no texture: Blockbench leaves the face out
        f = {'uv': face['uv'], 'texture': TEXTURES[face['texture']]}
        if face.get('rotation'):
            f['rotation'] = face['rotation']
        faces[name] = f
    out['faces'] = faces
    return out


def split(cube, most=32):
    """Pieces of a cube at most `most` pixels tall, cut along its own y: the
    rotation and origin stay, so together they are the same shape; side faces
    get their share of the texture (v runs from the top down), the top face
    goes with the highest piece and the bottom face with the lowest."""
    low, high = sorted((cube['from'][1], cube['to'][1]))
    if high - low <= most:
        return [cube]
    cuts = [low]
    while cuts[-1] + most < high:
        cuts.append(cuts[-1] + most)
    cuts.append(high)
    pieces = []
    for i, (a, b) in enumerate(zip(cuts, cuts[1:])):
        piece = dict(cube, name=f"{cube['name']}_{i}")
        piece['from'] = [cube['from'][0], a, cube['from'][2]]
        piece['to'] = [cube['to'][0], b, cube['to'][2]]
        faces = {}
        for name, face in cube['faces'].items():
            if name == 'up' and b != high or name == 'down' and a != low:
                continue
            if name in ('north', 'south', 'east', 'west'):
                turn = face.get('rotation') or 0
                assert turn in (0, 180), cube['name']
                u1, v1, u2, v2 = face['uv']
                if turn == 0:
                    at = lambda y: v1 + (high - y) / (high - low) * (v2 - v1)
                    face = dict(face, uv=[u1, at(b), u2, at(a)])
                else:
                    # Turned half way: the texture's top is the cube's bottom.
                    at = lambda y: v1 + (y - low) / (high - low) * (v2 - v1)
                    face = dict(face, uv=[u1, at(a), u2, at(b)])
            faces[name] = face
        piece['faces'] = faces
        pieces.append(piece)
    return pieces


def shift_for(cube):
    """Whole blocks to move a cube by, per axis, so its corners are within -16 to 32."""
    k = []
    for axis in range(3):
        low, high = sorted((cube['from'][axis], cube['to'][axis]))
        if low < -16:
            n = math.ceil((-16 - low) / 16)
        elif high > 32:
            n = -math.ceil((high - 32) / 16)
        else:
            n = 0
        assert -16 <= low + 16 * n and high + 16 * n <= 32, cube['name']
        k.append(n)
    return tuple(k)


def main(up):
    src = up / 'src/generators/resources/assets' / NS / 'models/block'
    upstream = json.loads((src / 'wind_generator.json').read_text(encoding='utf-8'))
    base = upstream['children']['base']
    bb = json.loads((src / 'wind_generator.bbmodel').read_text(encoding='utf-8'))
    base_names = {e['name'] for e in base['elements']}

    model = {'parent': 'block/block', 'textures': TEXTURE_IDS, 'elements': base['elements']}
    for key in ('display', 'gui_light'):
        if key in upstream:
            model[key] = upstream[key]
    (A / 'models/block').mkdir(parents=True, exist_ok=True)
    (A / 'models/block/wind_generator.json').write_text(json.dumps(model, indent=2) + '\n', encoding='utf-8')

    parts = {}
    hub = None
    for cube in bb['elements']:
        if cube['name'] in base_names or cube['name'].startswith('port'):
            continue
        kind = 'rotor' if cube['name'].startswith(BLADES) else 'tower'
        for piece in split(cube):
            k = shift_for(piece)
            parts.setdefault((kind, k), []).append(element(piece, [16 * n for n in k]))
        if cube['name'] == 'blade_center':
            # Blockbench's x -180, y -180 turns (x, y, z) into (-x, -y, z) about the origin.
            x = -(cube['from'][0] + cube['to'][0]) / 2
            y = -(cube['from'][1] + cube['to'][1]) / 2
            z = (cube['from'][2] + cube['to'][2]) / 2
            hub = [x / 16, y / 16, z / 16]
    out = A / 'models/block/wind_generator'
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    listed = []
    for i, ((kind, k), elements) in enumerate(sorted(parts.items())):
        name = f'{kind}_{i}'
        (out / f'{name}.json').write_text(json.dumps({'textures': TEXTURE_IDS, 'elements': elements}, indent=2) + '\n',
                                          encoding='utf-8')
        listed.append({'model': f'{NS}:block/wind_generator/{name}', 'kind': kind, 'moved': list(k)})
    (ROOT / 'scripts/wind_parts.json').write_text(json.dumps({'hub': hub, 'parts': listed}, indent=2) + '\n',
                                                  encoding='utf-8')

    tex = up / 'src/generators/resources/assets' / NS / 'textures/block/model/wind_generator.png'
    dst = A / 'textures/block/model/wind_generator.png'
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(tex, dst)
    print(f'wind generator: {len(base["elements"])} base elements, parts {[p["model"] for p in listed]}, hub {hub}')


if __name__ == '__main__':
    main(Path(sys.argv[1]))
