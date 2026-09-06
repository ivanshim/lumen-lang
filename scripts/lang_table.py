#!/usr/bin/env python3
"""Render the language definitions in langs/ as one comparison table.

Reads every langs/*.json, checks that all files carry the same labels in
the same order, and rewrites the table between the markers in
langs/README.md. Exit status is non-zero if the files disagree on their
labels, so the table can never describe a key one file lacks.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANGS = ROOT / "langs"
README = LANGS / "README.md"
START, END = "<!-- table:start -->", "<!-- table:end -->"
FIRST = ["lumen", "python", "php", "rust"]


def code(s: str) -> str:
    """A code span for a table cell; a pipe must be escaped there."""
    return "`" + s.replace("|", "\\|") + "`"


def plain(s: str) -> str:
    """A code span outside a table, where a backslash would show literally."""
    return "`" + s + "`"


def cell(value) -> str:
    if isinstance(value, list):
        return " ".join(code(v) for v in value) if value else "-"
    if value is None:
        return "-"
    if isinstance(value, bool):
        return code("true" if value else "false")
    return code(str(value))


def tiers(value) -> str:
    return " < ".join(" ".join(plain(op) for op in tier) for tier in value)


def main() -> int:
    files = sorted(LANGS.glob("*.json"), key=lambda p: (FIRST.index(p.stem) if p.stem in FIRST else len(FIRST), p.stem))
    langs = {p.stem: json.loads(p.read_text(encoding="utf-8")) for p in files}
    orders = {name: [k for k in data if not k.startswith("$comment")] for name, data in langs.items()}
    reference = next(iter(orders.values()))
    for name, order in orders.items():
        if order != reference:
            missing = sorted(set(reference) - set(order))
            extra = sorted(set(order) - set(reference))
            print(f"{name}.json disagrees on labels: missing {missing}, extra {extra}, or different order", file=sys.stderr)
            return 1

    names = list(langs)
    lines = ["| Label | " + " | ".join(names) + " |", "|---|" + "---|" * len(names)]
    for key in reference:
        if key == "op.precedence":
            continue
        lines.append("| " + code(key) + " | " + " | ".join(cell(langs[n][key]) for n in names) + " |")
    lines.append("")
    lines.append("Operator precedence, lowest tier first. Unary operators sit in their own tier.")
    lines.append("")
    for n in names:
        lines.append(f"- **{n}**: {tiers(langs[n]['op.precedence'])}")
    table = "\n".join(lines)

    text = README.read_text(encoding="utf-8")
    head, _, rest = text.partition(START)
    _, _, tail = rest.partition(END)
    README.write_text(f"{head}{START}\n{table}\n{END}{tail}", encoding="utf-8")
    print(f"language table: {len(names)} languages, {len(reference)} labels")
    return 0


if __name__ == "__main__":
    sys.exit(main())
