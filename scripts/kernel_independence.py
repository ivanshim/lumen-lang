#!/usr/bin/env python3
"""Fail if the two kernel crates depend on each other or share code.

Checks:
  1. neither crate names the other;
  2. no run of MIN_RUN or more significant, whitespace-normalised source
     lines appears in both trees.
Shared vocabulary (a struct called Token in both) is not shared code; only
identical line runs are.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STREAM = ROOT / "kernels/stream/src"
MICROCODE = ROOT / "kernels/microcode/src"
MIN_RUN = 5


def normalised(line: str) -> str:
    return re.sub(r"\s+", " ", re.sub(r"//.*$", "", line).strip())


def significant(line: str) -> bool:
    return len(line) > 12 and not line.startswith(("use ", "pub mod ", "mod ")) and line not in {"} else {", "});"}


def lines(tree: Path):
    out = []
    for path in sorted(tree.rglob("*.rs")):
        for number, raw in enumerate(path.read_text().split("\n"), 1):
            text = normalised(raw)
            if significant(text):
                out.append((path.relative_to(ROOT), number, text))
    return out


def main() -> int:
    problems = []
    for tree, other in ((STREAM, "lumen_microcode"), (MICROCODE, "lumen_stream")):
        for path in tree.rglob("*.rs"):
            if other in path.read_text():
                problems.append(f"{path.relative_to(ROOT)} refers to {other}")

    a, b = lines(STREAM), lines(MICROCODE)
    windows = {}
    for i in range(len(a) - MIN_RUN + 1):
        windows.setdefault(tuple(x[2] for x in a[i : i + MIN_RUN]), i)
    j = 0
    while j <= len(b) - MIN_RUN:
        key = tuple(x[2] for x in b[j : j + MIN_RUN])
        if key in windows:
            i, n = windows[key], MIN_RUN
            while i + n < len(a) and j + n < len(b) and a[i + n][2] == b[j + n][2]:
                n += 1
            problems.append(f"{n} identical lines: {a[i][0]}:{a[i][1]} and {b[j][0]}:{b[j][1]}")
            j += n
        else:
            j += 1

    shared = len({x[2] for x in a} & {x[2] for x in b})
    print(f"kernel independence: {len(a)} stream lines, {len(b)} microcode lines, {shared} coincide, {len(problems)} problem(s)")
    for p in problems:
        print(f"  {p}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
