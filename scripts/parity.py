"""Parity list: upstream Mekanism 26.3 vs. the four Horizon content mods.

Compares, per mod (mekanism, mekanismgenerators, mekanismtools, mekanismadditions)
and category (items, blocks, recipes, tags, models, textures, lang, loot), what
upstream ships with what the content mods under `content/` contain, and writes

- docs/parity.json  machine-readable list: original id -> Horizon id -> status
- docs/PARITY.md    counts per mod and category, top gaps

Statuses: done, partial, blocked, missing.
- items:    done = registered (horizon/item or horizon/block with "item") and has an
            item definition; partial = only one of the two.
- blocks:   done = registered and has every blockstate/loot table upstream has;
            partial = registered but one of these is missing.
- recipes, models, textures, loot: done = file with the same id exists in a content mod.
- tags:     done = every upstream member that can exist here (non-Mekanism ids, or
            Mekanism ids registered here) is in the local tag; partial = some are;
            missing = no local tag file (or no member can exist here yet).
- lang:     done = same key in the mod's en_us.json.
- blocked:  a missing entry matching BLOCKERS below (reasons quoted from TODO.md).

parity.json stores the upstream facts it needs (block blockstate/loot flags, tag members),
so `--check` re-evaluates the committed list against the current `content/` without an
upstream checkout and fails when an entry that was `done` is no longer `done`.

Usage:
  python scripts/parity.py --upstream <Mekanism 26.3 checkout>   regenerate docs/parity.json + PARITY.md
  python scripts/parity.py --check                               regression check (CI), no upstream needed
Options: --strict (with --upstream: exit 1 on regressions too), --out-json/--out-md.

Upstream needs a sparse checkout of src/{main,generators,tools,additions}/resources and
src/datagen/generated (same as scripts/import_upstream.py plus the three small mods).
"""
from collections import Counter, defaultdict
from pathlib import Path
import argparse
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
MODS = ['mekanism', 'mekanismgenerators', 'mekanismtools', 'mekanismadditions']
# Upstream source roots of each mod, relative to the checkout's src/.
SOURCES = {
    'mekanism': ['datagen/generated/mekanism', 'main/resources'],
    'mekanismgenerators': ['datagen/generated/mekanismgenerators', 'generators/resources'],
    'mekanismtools': ['datagen/generated/mekanismtools', 'tools/resources'],
    'mekanismadditions': ['datagen/generated/mekanismadditions', 'additions/resources'],
}
# Namespaces whose tags are compared (other namespaces in datagen are integration compat).
TAG_NS = set(MODS) | {'c', 'minecraft', 'neoforge'}
CATEGORIES = ['items', 'blocks', 'recipes', 'tags', 'models', 'textures', 'lang', 'loot']
STATUSES = ['done', 'partial', 'blocked', 'missing']

# (id regex, categories, reason). Only applied to entries that would otherwise be `missing`.
# Reasons are the blocker notes in TODO.md.
BLOCKERS = [
    (r'^mekanism:(basic|advanced|elite|ultimate)_universal_cable$', {'items', 'blocks'},
     'Horizon native API has no energy helpers for mods (TODO M05)'),
    (r'^mekanism:energized_smelter$', {'items', 'blocks'},
     'Horizon gives native mods no minecraft:smelting recipes (TODO M07)'),
]


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8'))


def tag_member(raw):
    """Entry of a tag file: string or {"id": ..., "required": ...}."""
    return raw['id'] if isinstance(raw, dict) else raw


# --- upstream scan -------------------------------------------------------

def scan_upstream(src):
    """-> {mod: {category: {id: facts}}}; facts is {} or small dict (blocks, tags)."""
    out = {m: {c: {} for c in CATEGORIES} for m in MODS}
    for mod, roots in SOURCES.items():
        cats = out[mod]
        states, loots = set(), set()
        for rel in roots:
            root = src / rel
            if not root.is_dir():
                sys.exit(f'upstream folder missing: {root}')
            a = root / 'assets' / mod
            d = root / 'data' / mod
            for p in (a / 'items').glob('*.json'):
                cats['items'][f'{mod}:{p.stem}'] = {}
            for p in (a / 'blockstates').glob('*.json'):
                states.add(p.stem)
            for p in (d / 'loot_table').rglob('*.json'):
                key = p.relative_to(d / 'loot_table').with_suffix('').as_posix()
                cats['loot'][f'{mod}:{key}'] = {}
                if key.startswith('blocks/'):
                    loots.add(key[len('blocks/'):])
            for p in (a / 'models').rglob('*.json'):
                cats['models'][f'{mod}:{p.relative_to(a / "models").with_suffix("").as_posix()}'] = {}
            for p in (a / 'textures').rglob('*.png'):
                cats['textures'][f'{mod}:{p.relative_to(a / "textures").with_suffix("").as_posix()}'] = {}
            for p in (d / 'recipe').rglob('*.json'):
                cats['recipes'][f'{mod}:{p.relative_to(d / "recipe").with_suffix("").as_posix()}'] = {}
            lang = a / 'lang' / 'en_us.json'
            if lang.is_file():
                for k in read_json(lang):
                    cats['lang'][k] = {}
            for tagdir in (root / 'data').glob('*/tags'):
                ns = tagdir.parent.name
                if ns not in TAG_NS:
                    continue
                for p in tagdir.rglob('*.json'):
                    rel = p.relative_to(tagdir).with_suffix('').as_posix()  # <type>/<path>
                    tid = f'{ns}:{rel}'
                    members = [tag_member(v) for v in read_json(p).get('values', [])]
                    old = cats['tags'].get(tid, {}).get('members', [])
                    cats['tags'][tid] = {'members': sorted(set(old) | set(members))}
        for b in sorted(states | loots):
            cats['blocks'][f'{mod}:{b}'] = {'state': b in states, 'loot': b in loots}
    return out


# --- local scan ----------------------------------------------------------

def scan_local():
    """-> per category sets of ids found in content/*; plus registered ids."""
    loc = {c: {} for c in CATEGORIES}          # category -> id -> relative file
    loc['state'], loc['itemdef'] = {}, {}
    reg_items, reg_blocks = {}, {}
    lang = {m: set() for m in MODS}
    for mod in MODS:
        base = CONTENT / mod
        for ns_dir in (base / 'data').glob('*'):
            ns = ns_dir.name
            for p in (ns_dir / 'horizon' / 'item').glob('*.json'):
                reg_items[f'{ns}:{p.stem}'] = p
            for p in (ns_dir / 'horizon' / 'block').glob('*.json'):
                reg_blocks[f'{ns}:{p.stem}'] = p
                if read_json(p).get('item'):
                    reg_items.setdefault(f'{ns}:{p.stem}', p)
            for p in (ns_dir / 'recipe').rglob('*.json'):
                loc['recipes'][f'{ns}:{p.relative_to(ns_dir / "recipe").with_suffix("").as_posix()}'] = p
            for p in (ns_dir / 'loot_table').rglob('*.json'):
                loc['loot'][f'{ns}:{p.relative_to(ns_dir / "loot_table").with_suffix("").as_posix()}'] = p
            for p in (ns_dir / 'tags').rglob('*.json'):
                tid = f'{ns}:{p.relative_to(ns_dir / "tags").with_suffix("").as_posix()}'
                loc['tags'].setdefault(tid, []).append(p)
        for ns_dir in (base / 'assets').glob('*'):
            ns = ns_dir.name
            for p in (ns_dir / 'items').glob('*.json'):
                loc['itemdef'][f'{ns}:{p.stem}'] = p
            for p in (ns_dir / 'blockstates').glob('*.json'):
                loc['state'][f'{ns}:{p.stem}'] = p
            for p in (ns_dir / 'models').rglob('*.json'):
                loc['models'][f'{ns}:{p.relative_to(ns_dir / "models").with_suffix("").as_posix()}'] = p
            for p in (ns_dir / 'textures').rglob('*.png'):
                loc['textures'][f'{ns}:{p.relative_to(ns_dir / "textures").with_suffix("").as_posix()}'] = p
            lang_file = ns_dir / 'lang' / 'en_us.json'
            if lang_file.is_file() and ns in lang:
                lang[ns] |= set(read_json(lang_file))
    loc['reg_items'], loc['reg_blocks'], loc['langkeys'] = reg_items, reg_blocks, lang
    # tag id -> members present locally (union over the mods)
    tagmembers = {}
    for tid, files in loc['tags'].items():
        s = set()
        for p in files:
            s |= {tag_member(v) for v in read_json(p).get('values', [])}
        tagmembers[tid] = s
    loc['tagmembers'] = tagmembers
    return loc


def rel(p):
    p = p[0] if isinstance(p, list) else p
    return p.relative_to(ROOT).as_posix()


# --- evaluation ----------------------------------------------------------

def expected_tag_members(tid, members, loc):
    """Upstream tag members that can exist here."""
    ttype = tid.split(':', 1)[1].split('/', 1)[0]
    out = []
    for m in members:
        ns = m.lstrip('#').split(':', 1)[0]
        if ns not in MODS:
            out.append(m)
        elif m.startswith('#'):
            if m[1:].replace(':', ':' + ttype + '/', 1) in loc['tagmembers']:
                out.append(m)
        elif ttype == 'item' and m in loc['reg_items']:
            out.append(m)
        elif ttype == 'block' and m in loc['reg_blocks']:
            out.append(m)
    return out


def evaluate(cat, mod, eid, facts, loc):
    """-> (status, horizon id or None, local file or None, note)."""
    if cat == 'items':
        reg, d = eid in loc['reg_items'], eid in loc['itemdef']
        file = loc['itemdef'].get(eid) or loc['reg_items'].get(eid)
        if reg and d:
            return 'done', eid, rel(file), ''
        if reg or d:
            return 'partial', eid, rel(file), 'no item definition' if reg else 'not registered'
    elif cat == 'blocks':
        reg = eid in loc['reg_blocks']
        if reg:
            lost = [n for n, k in (('blockstate', 'state'), ('loot table', 'loot'))
                    if facts.get(k) and (eid not in loc['state'] if k == 'state'
                                         else f'{eid.split(":")[0]}:blocks/{eid.split(":")[1]}' not in loc['loot'])]
            if not lost:
                return 'done', eid, rel(loc['reg_blocks'][eid]), ''
            return 'partial', eid, rel(loc['reg_blocks'][eid]), 'no ' + ', '.join(lost)
    elif cat in ('recipes', 'models', 'textures', 'loot'):
        if eid in loc[cat]:
            return 'done', eid, rel(loc[cat][eid]), ''
    elif cat == 'tags':
        exp = expected_tag_members(eid, facts.get('members', []), loc)
        have = loc['tagmembers'].get(eid)
        if have is not None:
            miss = [m for m in exp if m not in have]
            file = rel(loc['tags'][eid])
            if not miss:
                return 'done', eid, file, ''
            return ('partial', eid, file, f'{len(miss)}/{len(exp)} members missing') if len(miss) < len(exp) \
                else ('missing', None, None, 'no portable member present')
        return 'missing', None, None, ('no local tag' if exp else 'no portable member yet')
    elif cat == 'lang':
        if eid in loc['langkeys'][mod]:
            return 'done', eid, f'content/{mod}/assets/{mod}/lang/en_us.json', ''
    for pat, cats, reason in BLOCKERS:
        if cat in cats and re.search(pat, eid):
            return 'blocked', None, None, reason
    return 'missing', None, None, ''


def build(upstream, loc):
    entries = []
    for mod in MODS:
        for cat in CATEGORIES:
            for eid in sorted(upstream[mod][cat]):
                facts = upstream[mod][cat][eid]
                st, hid, file, note = evaluate(cat, mod, eid, facts, loc)
                e = {'id': eid, 'mod': mod, 'cat': cat, 'horizon': hid, 'status': st}
                if file:
                    e['file'] = file
                if note:
                    e['note'] = note
                e.update(facts)
                entries.append(e)
    return entries


def entries_to_upstream(entries):
    up = {m: {c: {} for c in CATEGORIES} for m in MODS}
    for e in entries:
        up[e['mod']][e['cat']][e['id']] = {k: e[k] for k in ('state', 'loot', 'members') if k in e}
    return up


# --- output --------------------------------------------------------------

def counts(entries):
    t = defaultdict(Counter)
    for e in entries:
        t[(e['mod'], e['cat'])][e['status']] += 1
    return t


def write_json(path, meta, entries):
    lines = ['{', f'  "meta": {json.dumps(meta, ensure_ascii=False)},', '  "entries": [']
    lines += [('    ' + json.dumps(e, ensure_ascii=False, separators=(',', ':')) + (',' if i < len(entries) - 1 else ''))
              for i, e in enumerate(entries)]
    lines += ['  ]', '}']
    path.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


def pct(c):
    n = sum(c.values())
    return f'{100 * c["done"] / n:.0f}%' if n else '-'


def write_md(path, meta, entries):
    t = counts(entries)
    L = ['# Parity: Mekanism 26.3 vs. Horizon', '',
         'Generated by `python scripts/parity.py --upstream <checkout>`; do not edit. Machine-readable list: '
         '[`parity.json`](parity.json) (`id` = original id, `horizon` = Horizon id, `status`). '
         f'Upstream: `{meta["upstream_commit"]}`. Regression check: `python scripts/parity.py --check`.', '',
         'Status: **done** = complete here; **partial** = some parts exist; **blocked** = missing, blocker named in TODO.md; '
         '**missing** = not ported yet. Not compared: advancements, sounds, particles, shaders, Java-side registries '
         '(chemicals, entities, tile entities) and integration compat data (ae2, create, ...).', '']
    tot = Counter(e['status'] for e in entries)
    L += [f'Total: {len(entries)} entries, {tot["done"]} done ({pct(tot)}), {tot["partial"]} partial, '
          f'{tot["blocked"]} blocked, {tot["missing"]} missing.', '']
    for mod in MODS:
        L += [f'## {mod}', '', '| Category | Total | Done | Partial | Blocked | Missing | Done % |', '|---|---:|---:|---:|---:|---:|---:|']
        s = Counter()
        for cat in CATEGORIES:
            c = t[(mod, cat)]
            s += c
            L.append(f'| {cat} | {sum(c.values())} | {c["done"]} | {c["partial"]} | {c["blocked"]} | {c["missing"]} | {pct(c)} |')
        L += [f'| **all** | {sum(s.values())} | {s["done"]} | {s["partial"]} | {s["blocked"]} | {s["missing"]} | {pct(s)} |', '']
    L += ['## Top gaps', '', 'Largest open categories (partial + blocked + missing):', '']
    gaps = sorted(((sum(c.values()) - c['done'], k) for k, c in t.items() if sum(c.values()) > c['done']), reverse=True)
    L += [f'- {mod} / {cat}: {n} of {sum(t[(mod, cat)].values())}' for n, (mod, cat) in gaps[:10]] + ['']
    for title, st, cats in (('Partial blocks and items', 'partial', ('blocks', 'items')),
                            ('Blocked', 'blocked', CATEGORIES)):
        sel = [e for e in entries if e['status'] == st and e['cat'] in cats]
        if sel:
            L += [f'### {title} ({len(sel)})', '']
            L += [f'- `{e["id"]}` ({e["cat"]}): {e.get("note", "")}' for e in sel[:40]]
            L += [f'- ... {len(sel) - 40} more in parity.json' if len(sel) > 40 else '', '']
    for mod in MODS:
        miss = [e for e in entries if e['mod'] == mod and e['cat'] == 'blocks' and e['status'] == 'missing']
        if miss:
            groups = Counter(re.sub(r'_(\w+)$', '', e['id'].split(':')[1]) if mod == 'mekanismadditions'
                             else e['id'].split(':')[1] for e in miss)
            L += [f'### Missing blocks: {mod} ({len(miss)})', '']
            if mod == 'mekanismadditions':
                L += ['By family (colour suffix stripped): ' + ', '.join(f'{k} x{v}' for k, v in groups.most_common(12)), '']
            else:
                L += [', '.join(f'`{e["id"].split(":")[1]}`' for e in miss[:60]) + (' ...' if len(miss) > 60 else ''), '']
    path.write_text('\n'.join(L).rstrip('\n') + '\n', encoding='utf-8', newline='\n')


# --- main ----------------------------------------------------------------

def regressions(old_entries, new_entries):
    new = {(e['mod'], e['cat'], e['id']): e['status'] for e in new_entries}
    return [(k, new.get(k, 'removed')) for k in ((e['mod'], e['cat'], e['id']) for e in old_entries
            if e['status'] == 'done') if new.get(k) != 'done']


def report_regressions(regs):
    for (mod, cat, eid), now in regs[:50]:
        print(f'REGRESSION {mod}/{cat}: {eid} was done, now {now}')
    if len(regs) > 50:
        print(f'... {len(regs) - 50} more')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--upstream', type=Path, help='Mekanism 26.3 checkout (regenerate outputs)')
    ap.add_argument('--check', action='store_true', help='re-evaluate docs/parity.json against content/, fail on regressions')
    ap.add_argument('--strict', action='store_true', help='with --upstream: exit 1 on regressions vs. the old parity.json')
    ap.add_argument('--out-json', type=Path, default=ROOT / 'docs' / 'parity.json')
    ap.add_argument('--out-md', type=Path, default=ROOT / 'docs' / 'PARITY.md')
    ap.add_argument('--upstream-commit', help='commit id to record (default: git rev-parse in the checkout)')
    a = ap.parse_args()
    if bool(a.upstream) == a.check:
        ap.error('give exactly one of --upstream or --check')
    old = read_json(a.out_json) if a.out_json.is_file() else None
    loc = scan_local()
    if a.check:
        if not old:
            sys.exit(f'{a.out_json} missing')
        entries = build(entries_to_upstream(old['entries']), loc)
        regs = regressions(old['entries'], entries)
        report_regressions(regs)
        gained = sum(1 for e, o in zip(entries, old['entries']) if e['status'] == 'done' and o['status'] != 'done')
        print(f'parity check: {len(old["entries"])} entries, {len(regs)} regressions, {gained} newly done '
              f'(regenerate with --upstream to refresh docs/parity.json)')
        return 1 if regs else 0
    src = a.upstream / 'src'
    commit = a.upstream_commit
    if not commit:
        try:
            commit = subprocess.run(['git', '-C', str(a.upstream), 'rev-parse', 'HEAD'], capture_output=True,
                                    text=True, check=True).stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            commit = old['meta']['upstream_commit'] if old else 'unknown'
    entries = build(scan_upstream(src), loc)
    meta = {'upstream': 'mekanism/Mekanism@26.3', 'upstream_commit': commit, 'mods': MODS, 'categories': CATEGORIES,
            'statuses': STATUSES, 'entries': len(entries)}
    regs = regressions(old['entries'], entries) if old else []
    report_regressions(regs)
    write_json(a.out_json, meta, entries)
    write_md(a.out_md, meta, entries)
    tot = Counter(e['status'] for e in entries)
    print(f'wrote {a.out_json.name} and {a.out_md.name}: {len(entries)} entries, ' + ', '.join(f'{s} {tot[s]}' for s in STATUSES))
    return 1 if regs and a.strict else 0


if __name__ == '__main__':
    sys.exit(main())
