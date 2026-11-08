# Coding agent notes for this Windows starter

- The repo root contains the older Linux-validated native mixin template. Do not silently replace it.
- Content source is in `content/mekanism/` (Horizon content API 0.2). Run `cargo horizon-mod build` **there**, never at repository root.
- Run `python scripts/validate_content.py` after every content change, then official `cargo horizon-mod check` when the tool is available.
- `logic/energy/` is an independent Rust library for tests only, not wired to game ticks, block entities or persistence.
- Do not claim machines, pipes or world generation are implemented. Prefer small testable additions.
- Native mod development: use `cargo horizon-mod new ... --template native` and the Horizon launcher on Windows, using current hook contracts.
- Do not copy Mekanism assets/code without preserving applicable licensing and attribution.
