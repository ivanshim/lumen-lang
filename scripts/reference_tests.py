#!/usr/bin/env python3
"""Run the reference test suites under tests/ against a definition and say
what the language, as defined here, does not yet do.

tests/php/ holds php-src's tests/lang, tests/basic and tests/func (.phpt
files: a --FILE-- section to run and an --EXPECT-- section to match);
tests/python/ holds the core-language files of CPython's Lib/test. Each
test is run on one kernel with the language's definition, its output is
compared with what the reference expects, and a test that fails is
classified by the first error the kernel reports. The report,
tests/REPORT.md, counts the results, ranks the reasons, and lists the
language's reserved words and the most-called functions of the suites
against what the definition spells: the missing pieces, in the order
the reference suites need them.

Usage: python3 scripts/reference_tests.py [--kernel stack8]
"""
import json
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "release" / "lumen-lang"
REPORT = ROOT / "tests" / "REPORT.md"
TIMEOUT = 10
CLOSING_TAGS = [0]

PHP_RESERVED = ("abstract and array as break callable case catch class clone const continue declare default do echo "
                "else elseif empty enddeclare endfor endforeach endif endswitch endwhile eval exit extends final finally fn "
                "for foreach function global goto if implements include include_once instanceof insteadof interface isset "
                "list match namespace new or print private protected public readonly require require_once return static "
                "switch throw trait try unset use var while xor yield").split()
PYTHON_KEYWORDS = ("False None True and as assert async await break class continue def del elif else except finally for "
                   "from global if import in is lambda nonlocal not or pass raise return try while with yield").split()


def spelled(definition):
    words = set()
    for label, value in definition.items():
        if label.startswith("$") or not isinstance(value, list):
            continue
        for v in value:
            if isinstance(v, str):
                words.add(v)
            elif isinstance(v, list):
                words.update(x for x in v if isinstance(x, str))
    return words


def run(args, source, suffix):
    with tempfile.NamedTemporaryFile("w", suffix=suffix, delete=False, encoding="utf-8") as f:
        f.write(source)
        path = f.name
    try:
        p = subprocess.run([str(BINARY)] + args + [path], capture_output=True, text=True, timeout=TIMEOUT, errors="replace")
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return 124, "", "timeout"
    finally:
        Path(path).unlink(missing_ok=True)


def normalise(message, banner):
    """An error's shape: the banner off, names and numbers folded, keywords kept."""
    m = message.strip()
    if m.startswith(banner + ": "):
        m = m[len(banner) + 2:]
    m = re.sub(r"\(line \d+\)", "", m).strip()
    m = re.sub(r" at \d+:\d+", "", m)
    m = re.sub(r"'\$[A-Za-z_][A-Za-z0-9_]*'", "'$name'", m)
    m = re.sub(r"'\d+(\.\d+)?'", "'<number>'", m)
    m = re.sub(r'"[^"]*"', '"..."', m)
    return m


def reason_of(code, out, err, banner):
    text = (out.strip().splitlines() or err.strip().splitlines() or ["(no output)"])
    if code == 124:
        return "timeout"
    for line in reversed(text):
        if line.startswith(banner + ": ") or "Error" in line:
            return normalise(line, banner)
    return normalise(text[-1], banner)


# ---------------------------------------------------------------- php

def phpt_sections(text):
    sections = {}
    name = None
    for line in text.split("\n"):
        m = re.fullmatch(r"--([A-Z_]+)--\s*", line)
        if m:
            name = m.group(1)
            sections[name] = []
        elif name:
            sections[name].append(line)
    return {k: "\n".join(v).rstrip("\n") for k, v in sections.items()}


def expectf_pattern(expected):
    out = []
    i = 0
    while i < len(expected):
        c = expected[i]
        if c == "%" and i + 1 < len(expected):
            k = expected[i + 1]
            i += 2
            out.append({"s": ".+?", "S": ".*?", "d": r"[+-]?\d+", "i": r"[+-]?\d+", "f": r"[+-]?\d+(\.\d+)?([eE][+-]?\d+)?",
                        "e": r".", "a": ".+?", "A": ".*?", "c": ".", "w": r"\s*", "x": r"[0-9a-fA-F]+", "%": "%"}.get(k, re.escape("%" + k)))
            continue
        out.append(re.escape(c))
        i += 1
    return "".join(out)


def run_phpt(path):
    s = phpt_sections(path.read_text(encoding="utf-8", errors="replace"))
    if "FILE" not in s:
        return "skipped", "no --FILE-- section"
    if "SKIPIF" in s and re.search(r"extension_loaded|PHP_OS|getenv|zend\.", s["SKIPIF"]):
        pass  # run anyway: the kernels have no extensions to check; the test shows what is missing
    expected = s.get("EXPECT", s.get("EXPECTF", s.get("EXPECTREGEX")))
    if expected is None:
        return "skipped", "no --EXPECT-- section"
    # A closing `?>` ends most reference files; no definition label spells it
    # yet, so it is taken off here and counted, and the test shows what comes next.
    source = s["FILE"]
    if re.search(r"\?>\s*$", source):
        source = re.sub(r"\?>\s*$", "", source)
        CLOSING_TAGS[0] += 1
    code, out, err = run(["--lang", "langs/extras/php.json"], source, ".php")
    got = out.rstrip()
    want = expected.rstrip()
    if code == 0:
        ok = (got == want) if "EXPECT" in s else (re.fullmatch(expectf_pattern(want) if "EXPECTF" in s else want, got, re.S) is not None)
        if ok:
            return "pass", ""
        return "differs", "ran, printed something else"
    return "error", reason_of(code, out, err, "PhpError")


# ---------------------------------------------------------------- python

def run_python(path):
    source = path.read_text(encoding="utf-8", errors="replace")
    code, out, err = run([], source, ".py")
    if code == 0:
        return "differs", "ran to the end without asserting anything"
    return "error", reason_of(code, out, err, "PythonError")


# ---------------------------------------------------------------- usage

def calls_in(text, exclude):
    return Counter(n for n in re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", text) if n not in exclude)


def keyword_table(words, have):
    yes = [w for w in words if w in have]
    no = [w for w in words if w not in have]
    return yes, no


def main():
    kernel = "stack8"
    if "--kernel" in sys.argv:
        kernel = sys.argv[sys.argv.index("--kernel") + 1]
    subprocess.run(["cargo", "build", "--release", "--quiet"], cwd=ROOT, check=True)
    php_def = json.loads((ROOT / "langs" / "extras" / "php.json").read_text())
    py_def = json.loads((ROOT / "langs" / "python.json").read_text())
    php_have, py_have = spelled(php_def), spelled(py_def)

    results = {}
    reasons = {"php": Counter(), "python": Counter()}
    php_files = sorted((ROOT / "tests" / "php").rglob("*.phpt"))
    py_files = sorted((ROOT / "tests" / "python").glob("*.py"))
    for f in php_files:
        status, why = run_phpt(f)
        results[f] = (status, why)
        if status in ("error", "differs", "skipped"):
            reasons["php"][why] += 1
    for f in py_files:
        status, why = run_python(f)
        results[f] = (status, why)
        reasons["python"][why] += 1

    php_calls = Counter()
    for f in php_files:
        s = phpt_sections(f.read_text(encoding="utf-8", errors="replace"))
        php_calls.update(calls_in(s.get("FILE", ""), PHP_RESERVED))
    py_calls = Counter()
    for f in py_files:
        py_calls.update(calls_in(f.read_text(encoding="utf-8", errors="replace"), PYTHON_KEYWORDS))

    def totals(files):
        c = Counter(results[f][0] for f in files)
        return c
    lines = ["# The reference suites against the definitions", "",
             "Generated by `scripts/reference_tests.py`. `tests/php/` is php-src's `tests/lang`,",
             "`tests/basic` and `tests/func`; `tests/python/` is the core-language part of",
             "CPython's `Lib/test`. Each test ran on the " + kernel + " kernel with the language's",
             "definition. A test *passes* when it prints what the reference expects, *differs*",
             "when it runs but prints something else, and *errors* when the kernel stops it;",
             "the error's first line, names and numbers folded, is its reason. The reasons,",
             "ranked, are the constructs the definition (or a kernel) does not yet spell, in",
             "the order the reference suites need them. An error is the first one met: a",
             "character the lexer does not know stops a file before any keyword in it is",
             "seen, so the reserved-word and function tables below say what lies behind it.", ""]
    for name, files in (("PHP", php_files), ("Python", py_files)):
        c = totals(files)
        lines += [f"## {name}: {len(files)} tests", "",
                  f"pass {c['pass']}, differs {c['differs']}, error {c['error']}, skipped {c['skipped']}", ""]
        if name == "PHP":
            lines += [f"{CLOSING_TAGS[0]} tests end with a closing `?>`, which no definition label spells; it is",
                      "taken off before the run so the test can show what it needs next.", ""]
        key = "php" if name == "PHP" else "python"
        lines += ["| Reason | Tests |", "|---|---|"]
        for why, n in reasons[key].most_common(40):
            lines.append("| " + why.replace("|", "\\|") + f" | {n} |")
        lines.append("")
        words = PHP_RESERVED if name == "PHP" else PYTHON_KEYWORDS
        have = php_have if name == "PHP" else py_have
        yes, no = keyword_table(words, have)
        lines += [f"### Reserved words: {len(yes)} of {len(words)} spelled", "",
                  "Spelled: " + ", ".join(f"`{w}`" for w in yes), "",
                  "Not spelled: " + ", ".join(f"`{w}`" for w in no), ""]
        calls = php_calls if name == "PHP" else py_calls
        lines += ["### Most-called functions in the suite, and whether the definition spells them", "",
                  "| Function | Calls | Spelled |", "|---|---|---|"]
        for fn, n in calls.most_common(40):
            lines.append(f"| `{fn}` | {n} | {'yes' if fn in have else 'no'} |")
        lines.append("")
    lines += ["## Every test", "", "| Test | Result | Reason |", "|---|---|---|"]
    for f in php_files + py_files:
        status, why = results[f]
        lines.append(f"| `{f.relative_to(ROOT / 'tests')}` | {status} | " + why.replace("|", "\\|") + " |")
    REPORT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    for name, files in (("PHP", php_files), ("Python", py_files)):
        c = totals(files)
        print(f"{name}: {len(files)} tests: pass {c['pass']}, differs {c['differs']}, error {c['error']}, skipped {c['skipped']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
