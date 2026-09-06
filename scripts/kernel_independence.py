#!/usr/bin/env python3
"""Fail if any two kernel crates depend on each other or copy each other.

Checks, for every pair of kernels:
  1. neither crate names the other;
  2. no run of MIN_RUN or more significant, whitespace-normalised source
     lines appears in both trees.
The kernels read the same definitions and solve the same problems, each in
its own way, so short coincidences are expected; only a long identical run,
which means an algorithm was copied rather than re-derived, is a problem.
"""
import re
import sys
from itertools import combinations
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KERNELS = {
    "stream": (ROOT / "kernels/stream/src", "lumen_stream"),
    "microcode": (ROOT / "kernels/microcode/src", "lumen_microcode"),
    "stack": (ROOT / "kernels/stack/src", "lumen_stack"),
}
MIN_RUN = 12


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


def copied_runs(a, b):
    """Runs of MIN_RUN or more identical significant lines in both trees."""
    windows = {}
    for i in range(len(a) - MIN_RUN + 1):
        windows.setdefault(tuple(x[2] for x in a[i : i + MIN_RUN]), i)
    found = []
    j = 0
    while j <= len(b) - MIN_RUN:
        key = tuple(x[2] for x in b[j : j + MIN_RUN])
        if key in windows:
            i, n = windows[key], MIN_RUN
            while i + n < len(a) and j + n < len(b) and a[i + n][2] == b[j + n][2]:
                n += 1
            found.append(f"{n} identical lines: {a[i][0]}:{a[i][1]} and {b[j][0]}:{b[j][1]}")
            j += n
        else:
            j += 1
    return found


def main() -> int:
    problems = []
    sources = {name: lines(tree) for name, (tree, _) in KERNELS.items()}
    for name, (tree, _) in KERNELS.items():
        for other, (_, crate) in KERNELS.items():
            if other == name:
                continue
            for path in tree.rglob("*.rs"):
                if crate in path.read_text():
                    problems.append(f"{path.relative_to(ROOT)} refers to {crate}")
    summary = []
    for a, b in combinations(KERNELS, 2):
        problems.extend(copied_runs(sources[a], sources[b]))
        shared = len({x[2] for x in sources[a]} & {x[2] for x in sources[b]})
        summary.append(f"{a}/{b} {shared} coincide")
    counts = ", ".join(f"{len(sources[k])} {k} lines" for k in KERNELS)
    print(f"kernel independence: {counts}; {'; '.join(summary)}; {len(problems)} problem(s)")
    for p in problems:
        print(f"  {p}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
