"""Turn Mekanism's Wind and Advanced Solar Generator models into plain models Horizon can draw.

Horizon reads neither NeoForge's composite and OBJ loaders nor a model's root
`transform`, and a model's elements must stay within -16 to 32. Run this after
`scripts/import_upstream.py`, which copies the upstream files this replaces.

Wind Generator: upstream draws it with the composite and OBJ loaders
(`models/block/wind_generator.json`: a `base` of ordinary elements and the
tower as `wind_generator.obj`) and turns the blades with its block entity
renderer. The Blockbench sources next to the OBJ files
(`wind_generator.bbmodel`, `java_block` format) hold the same geometry as
ordinary cubes, so this writes:

- `models/block/wind_generator.json`: the base, the block's own model;
- `models/block/wind_generator/tower_<i>.json` and `rotor_<i>.json`: the
  tower and the blades, each cube moved by whole blocks so it is within range
  (the native mod draws each part moved back, and turns the rotor about its
  hub; `scripts/wind_parts.json` lists the parts, how far each was moved per
  axis, and the hub);
- the textures these use.

Advanced Solar Generator: upstream's model stands one block low and is lifted
by its root `transform` (translation 0, 1, 0). This writes
`models/block/advanced_solar_generator.json` with what lies below the main
block's top, lifted one block (the foot and the lower pole), and
`models/block/advanced_solar_generator/top.json` with the rest as it is, which
the native mod draws one block up.

Both items get a plain model of the whole generator, shrunk into one block
(`models/item/<id>.json`), in place of upstream's special item renderer.

Mekanism is MIT licensed (content/mekanismgenerators/LICENSE-MEKANISM).
Usage: python scripts/import_generator_models.py <path to Mekanism checkout, branch 26.3>
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


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8')


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


def cut(cube, cuts):
    """Pieces of a cube (Blockbench cube or model element) cut along its own y
    at `cuts` (from its bottom to its top): the rotation and origin stay, so
    together they are the same shape; side faces get their share of the
    texture (v runs from the top down), the top face goes with the highest
    piece and the bottom face with the lowest."""
    low, high = cuts[0], cuts[-1]
    if len(cuts) == 2:
        return [cube]
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


def split(cube, most=32):
    """Pieces of a cube at most `most` pixels tall (see `cut`)."""
    low, high = sorted((cube['from'][1], cube['to'][1]))
    cuts = [low]
    while cuts[-1] + most < high:
        cuts.append(cuts[-1] + most)
    cuts.append(high)
    return cut(cube, cuts)


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


def corners(e):
    """An element's eight corners, turned by its rotation (x, then y, then z;
    for the bounds only, so the order of the turns hardly matters)."""
    pts = [[x, y, z] for x in (e['from'][0], e['to'][0])
           for y in (e['from'][1], e['to'][1]) for z in (e['from'][2], e['to'][2])]
    rot = e.get('rotation')
    if not rot:
        return pts
    if 'axis' in rot:
        turns = [(rot['axis'], rot['angle'])]
    else:
        turns = [(a, rot.get(a, 0)) for a in 'xyz']
    o = rot['origin']
    out = []
    for p in pts:
        v = [p[i] - o[i] for i in range(3)]
        for axis, deg in turns:
            s, c = math.sin(math.radians(deg)), math.cos(math.radians(deg))
            i, j = {'x': (1, 2), 'y': (2, 0), 'z': (0, 1)}[axis]
            v[i], v[j] = v[i] * c - v[j] * s, v[i] * s + v[j] * c
        out.append([v[i] + o[i] for i in range(3)])
    return out


def shrunk(elements):
    """The elements scaled down into one block: centred in x and z, standing on its floor."""
    pts = [p for e in elements for p in corners(e)]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    s = 16 / max(hi[i] - lo[i] for i in range(3))
    centre = [(lo[0] + hi[0]) / 2, lo[1], (lo[2] + hi[2]) / 2]
    goal = [8, 0, 8]
    move = lambda p: [round(goal[i] + (p[i] - centre[i]) * s, 4) for i in range(3)]
    out = []
    for e in elements:
        e = dict(e, **{'from': move(e['from']), 'to': move(e['to'])})
        if 'rotation' in e:
            e['rotation'] = dict(e['rotation'], origin=move(e['rotation']['origin']))
        out.append(e)
    return out


def item(gid, textures, elements):
    """A plain item model of the whole generator, shrunk into one block."""
    write(A / f'models/item/{gid}.json', {'parent': 'block/block', 'textures': textures,
                                          'elements': shrunk(elements)})
    write(A / f'items/{gid}.json', {'model': {'type': 'minecraft:model', 'model': f'{NS}:item/{gid}'}})


def wind(src, up):
    upstream = json.loads((src / 'wind_generator.json').read_text(encoding='utf-8'))
    base = upstream['children']['base']
    bb = json.loads((src / 'wind_generator.bbmodel').read_text(encoding='utf-8'))
    base_names = {e['name'] for e in base['elements']}

    model = {'parent': 'block/block', 'textures': TEXTURE_IDS, 'elements': base['elements']}
    for key in ('display', 'gui_light'):
        if key in upstream:
            model[key] = upstream[key]
    write(A / 'models/block/wind_generator.json', model)

    parts = {}
    whole = list(base['elements'])
    hub = None
    for cube in bb['elements']:
        if cube['name'] in base_names or cube['name'].startswith('port'):
            continue
        kind = 'rotor' if cube['name'].startswith(BLADES) else 'tower'
        whole.append(element(cube, [0, 0, 0]))
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
    listed = []
    for i, ((kind, k), elements) in enumerate(sorted(parts.items())):
        name = f'{kind}_{i}'
        write(out / f'{name}.json', {'textures': TEXTURE_IDS, 'elements': elements})
        listed.append({'model': f'{NS}:block/wind_generator/{name}', 'kind': kind, 'moved': list(k)})
    write(ROOT / 'scripts/wind_parts.json', {'hub': hub, 'parts': listed})
    item('wind_generator', TEXTURE_IDS, whole)

    tex = up / 'src/generators/resources/assets' / NS / 'textures/block/model/wind_generator.png'
    dst = A / 'textures/block/model/wind_generator.png'
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(tex, dst)
    print(f'wind generator: {len(base["elements"])} base elements, parts {[p["model"] for p in listed]}, hub {hub}')


def advanced_solar(src):
    upstream = json.loads((src / 'advanced_solar_generator.json').read_text(encoding='utf-8'))
    assert upstream.get('transform') == {'translation': [0, 1, 0]}, upstream.get('transform')
    low, top = [], []
    for e in upstream['elements']:
        bottom, high = sorted((e['from'][1], e['to'][1]))
        for piece in cut(e, [bottom, 16, high]) if bottom < 16 < high else [e]:
            if max(piece['from'][1], piece['to'][1]) <= 16:
                low.append(element_lifted(piece))
            else:
                top.append(piece)
    model = {'parent': 'block/block', 'textures': upstream['textures'], 'elements': low}
    if 'display' in upstream:
        model['display'] = upstream['display']
    write(A / 'models/block/advanced_solar_generator.json', model)
    write(A / 'models/block/advanced_solar_generator/top.json',
          {'textures': upstream['textures'], 'elements': top})
    item('advanced_solar_generator', upstream['textures'], upstream['elements'])
    print(f'advanced solar generator: {len(low)} elements in the block, {len(top)} on top')


def element_lifted(e):
    """A model element moved up one block (upstream's root transform)."""
    out = dict(e, **{'from': [e['from'][0], e['from'][1] + 16, e['from'][2]],
                     'to': [e['to'][0], e['to'][1] + 16, e['to'][2]]})
    if 'rotation' in e:
        o = e['rotation']['origin']
        out['rotation'] = dict(e['rotation'], origin=[o[0], o[1] + 16, o[2]])
    return out


def main(up):
    src = up / 'src/generators/resources/assets' / NS / 'models/block'
    wind(src, up)
    advanced_solar(src)


if __name__ == '__main__':
    main(Path(sys.argv[1]))
