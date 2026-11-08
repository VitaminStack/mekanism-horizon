#!/usr/bin/env python3
"""Synchronize README progress bars with checked tasks in TODO.md."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
TODO = ROOT / "TODO.md"
README = ROOT / "README.md"

TASK = re.compile(r"^\s*[-*]\s+\[([ xX])\](?:\s|$)")
MILESTONE = re.compile(r"^##\s+(M\d{2})\s+-\s+")
SECTION_GROUPS = [
    ("Projektbasis", ("M00",)),
    ("Energy Cube", ("M01",)),
    ("Rohstoffe & Energie", ("M02", "M03", "M04")),
    ("Maschinen & Chemie", ("M05", "M06")),
    ("Automation & Logistik", ("M07", "M08")),
    ("Multiblocks & Reaktoren", ("M09", "M10")),
    ("Tools & Additions", ("M11", "M12")),
    ("Tests & Releases", ("M13", "DONE")),
    ("Naechste Schritte", ("NEXT",)),
]
PROGRESS = re.compile(
    r"<!-- progress:start -->.*?<!-- progress:end -->", re.DOTALL
)


def read_tasks(text: str) -> dict[str, tuple[int, int]]:
    counters: dict[str, list[int]] = {}
    current: str | None = None
    for line in text.splitlines():
        if line.startswith("## "):
            milestone = MILESTONE.match(line)
            if milestone:
                current = milestone.group(1)
            elif line.startswith("## Die nächsten 10 "):
                current = "NEXT"
            elif line.startswith("## Definition of Done"):
                current = "DONE"
            else:
                current = None
        task = TASK.match(line)
        if not task:
            continue
        if current is None:
            raise ValueError(f"TODO checkbox outside a tracked section: {line}")
        counter = counters.setdefault(current, [0, 0])
        counter[1] += 1
        if task.group(1).lower() == "x":
            counter[0] += 1
    return {key: (v[0], v[1]) for key, v in counters.items()}


def render_progress(counters: dict[str, tuple[int, int]]) -> str:
    def sum_groups(keys: tuple[str, ...]) -> tuple[int, int]:
        return (
            sum(counters.get(key, (0, 0))[0] for key in keys),
            sum(counters.get(key, (0, 0))[1] for key in keys),
        )

    covered = {key for _, keys in SECTION_GROUPS for key in keys}
    unknown = set(counters) - covered
    if unknown:
        raise ValueError(f"TODO sections missing from progress mapping: {sorted(unknown)}")

    def bar(done: int, total: int) -> str:
        filled = round(20 * done / total) if total else 0
        return "█" * filled + "░" * (20 - filled)

    def row(label: str, done: int, total: int) -> str:
        percent = round(100 * done / total) if total else 0
        return f"{label:<27} {bar(done, total)} {percent:>3}%  ({done}/{total})"

    all_done, all_total = sum_groups(tuple(covered))
    fence = chr(96) * 3
    lines = [
        "<!-- progress:start -->",
        fence + "text",
        row("Gesamt", all_done, all_total),
        "",
    ]
    for name, keys in SECTION_GROUPS:
        done, total = sum_groups(keys)
        lines.append(row(name, done, total))
    lines.extend([fence, "<!-- progress:end -->"])
    return "\n".join(lines)


def main() -> None:
    counters = read_tasks(TODO.read_text(encoding="utf-8"))
    block = render_progress(counters)
    original = README.read_text(encoding="utf-8")
    updated, count = PROGRESS.subn(lambda _: block, original, count=1)
    if count != 1:
        raise ValueError("README progress markers are missing")
    if updated != original:
        README.write_text(updated, encoding="utf-8")
        print("README progress updated")
    else:
        print("README progress is already current")


if __name__ == "__main__":
    main()
