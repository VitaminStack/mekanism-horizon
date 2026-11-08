"""Offline structural sanity checks; does NOT replace official cargo horizon-mod check."""
from pathlib import Path
import json
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content' / 'mekanism'


def main():
    manifest = tomllib.loads((CONTENT / 'horizon-mod.toml').read_text(encoding='utf-8'))
    assert manifest['id'] == 'mekanism'
    assert manifest['api'] == '0.2'
    assert manifest['side'] == 'both'
    assert 'registries' in manifest['capabilities']
    assert not (CONTENT / 'Cargo.toml').exists(), 'Content mods must not have Cargo.toml in their root'
    json_files = sorted(CONTENT.rglob('*.json'))
    for path in json_files:
        json.loads(path.read_text(encoding='utf-8'))
    assert len(json_files) >= 10, 'Expected item, block, model, loot, recipe and language files'
    item = json.loads((CONTENT/'data/mekanism/horizon/item/osmium_ingot.json').read_text(encoding='utf-8'))
    block = json.loads((CONTENT/'data/mekanism/horizon/block/osmium_block.json').read_text(encoding='utf-8'))
    assert item['template'].startswith('minecraft:')
    assert block['template'].startswith('minecraft:')
    for name in ['osmium_ingot', 'osmium_block']:
        assert (CONTENT/f'assets/mekanism/items/{name}.json').exists(), f'Missing item model: {name}'
    print(f'OK: {len(json_files)} JSON-Dateien + Manifest geprueft (nur Strukturtest).')
    print(r'Fuer offizielle Pruefung: .\scripts\Build-Content.ps1')


if __name__ == '__main__':
    try:
        main()
    except Exception as exc:
        print(f'FEHLER: {exc}', file=sys.stderr)
        sys.exit(1)
