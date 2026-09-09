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
# The definitions the kernels take in at build time, in the order they
# are shown. Every other definition is read from disk at run time and is
# marked as such, whichever directory under langs/ it happens to sit in.
BUILT_IN = ["lumen", "rplumen", "python", "rust"]


def code(s: str) -> str:
    """A code span for a table cell; a pipe must be escaped there."""
    return "`" + s.replace("|", "\\|") + "`"


def plain(s: str) -> str:
    """A code span outside a table, where a backslash would show literally."""
    return "`" + s + "`"


def cell(value, provided=None) -> str:
    """A table cell; an empty label the language's library provides says so."""
    if isinstance(value, list):
        if value:
            return " ".join(code(v) for v in value)
        return f"(library: {code(provided)})" if provided else "-"
    if value is None:
        return "-"
    if isinstance(value, bool):
        return code("true" if value else "false")
    return code(str(value))


def tiers(value) -> str:
    return " < ".join(" ".join(plain(op) for op in tier) for tier in value)


def main() -> int:
    here = sorted(LANGS.glob("*.json"), key=lambda p: (BUILT_IN.index(p.stem) if p.stem in BUILT_IN else len(BUILT_IN), p.stem))
    beside = sorted((LANGS / "extras").glob("*.json"))
    named = lambda p: p.stem if p.stem in BUILT_IN else f"{p.stem} (extra)"
    langs = {named(p): json.loads(p.read_text(encoding="utf-8")) for p in here + beside}
    # Core labels must agree in name and order; ext.* labels are optional
    # extensions a definition may add, read by the full kernels only.
    orders = {name: [k for k in data if not k.startswith("$") and not k.startswith("ext.")] for name, data in langs.items()}
    extensions = sorted({k for data in langs.values() for k in data if k.startswith("ext.")})
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
        lines.append("| " + code(key) + " | " + " | ".join(cell(langs[n][key], langs[n].get("$library", {}).get(key)) for n in names) + " |")
    lines.append("")
    lines.append("Operator precedence, lowest tier first. Unary operators sit in their own tier.")
    lines.append("")
    for n in names:
        lines.append(f"- **{n}**: {tiers(langs[n]['op.precedence'])}")
    if extensions:
        lines += ["", "Extension labels, optional and read by the full kernels only (absent means empty or false):", "",
                  "| Label | " + " | ".join(names) + " |", "|---|" + "---|" * len(names)]
        for key in extensions:
            lines.append("| " + code(key) + " | " + " | ".join(cell(langs[n][key]) if key in langs[n] else "-" for n in names) + " |")
    table = "\n".join(lines)

    text = README.read_text(encoding="utf-8")
    head, _, rest = text.partition(START)
    _, _, tail = rest.partition(END)
    README.write_text(f"{head}{START}\n{table}\n{END}{tail}", encoding="utf-8")
    print(f"language table: {len(names)} languages, {len(reference)} labels, {len(extensions)} extension labels")
    return 0


if __name__ == "__main__":
    sys.exit(main())
