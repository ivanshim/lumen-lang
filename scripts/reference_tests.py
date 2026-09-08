#!/usr/bin/env python3
"""Run the reference test suites under tests/ against a definition and say
what the language, as defined here, does not yet do.

tests/php/ holds php-src's tests/lang, tests/basic and tests/func (.phpt
files: a --FILE-- section to run and an --EXPECT-- section to match);
tests/python/ holds the core-language files of CPython's Lib/test. Each
test is run on the full kernels with the language's definition, its output is
compared with what the reference expects, and a test that fails is
classified by the first error the kernel reports. The report,
tests/REPORT.md, counts the results, ranks the reasons, and lists the
language's reserved words and the most-called functions of the suites
against what the definition spells: the missing pieces, in the order
the reference suites need them.

Usage: python3 scripts/reference_tests.py [--kernel stack8]

Without --kernel every test runs on both full kernels, stack8 and
microcode7, and the report shows them side by side.
"""
import json
import re
import os
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "release" / "lumen-lang"
REPORT = ROOT / "tests" / "REPORT.md"
TIMEOUT = 10
# The full kernels, the deployment one first: its reasons lead the report.
KERNELS = ["stack8", "microcode7"]

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


def run(kernel, args, source, suffix, request=None, beside=None):
    # php-src's run-tests.php writes the program next to the .phpt it came
    # from, so a test naming a file beside itself finds it. Where a test
    # says where it belongs, put it there; otherwise anywhere will do.
    if beside is not None and not beside.with_suffix(suffix).exists():
        path = str(beside.with_suffix(suffix))
        Path(path).write_text(source, encoding="utf-8")
    else:
        with tempfile.NamedTemporaryFile("w", suffix=suffix, delete=False, encoding="utf-8") as f:
            f.write(source)
            path = f.name
    setting = {**os.environ, **(request or {}).get("env", {})}
    body = (request or {}).get("body", "")
    try:
        p = subprocess.run([str(BINARY), "--kernel", kernel] + args + [path], input=body, capture_output=True,
                           text=True, timeout=TIMEOUT, errors="replace", env=setting)
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
    # Where a language names the file and line a fault happened in, the
    # file is a fresh temporary each run, so the shape stands for it.
    m = re.sub(r"\s+in\s+\S+?\.php(:\d+| on line \d+)", " in <file>", m)
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


def web_request(sections):
    """The request a .phpt describes: its --GET--, --POST--, --COOKIE-- and
    --ENV-- sections, given the way a web server gives one."""
    env = {"REQUEST_METHOD": "GET", "QUERY_STRING": sections.get("GET", "").strip()}
    for line in sections.get("ENV", "").splitlines():
        name, _, value = line.partition("=")
        if name.strip():
            env[name.strip()] = value.strip()
    # run-tests.php starts the run with the settings a test's INI
    # section names. The host carries them in the environment, each
    # under its own name with PHP_INI_ before it.
    for line in sections.get("INI", "").splitlines():
        name, _, value = line.partition("=")
        if name.strip() and not name.strip().startswith(";"):
            env["PHP_INI_" + name.strip()] = value.strip()
    cookie = sections.get("COOKIE", "").strip()
    if cookie:
        env["HTTP_COOKIE"] = cookie
    body = sections.get("POST", "").strip()
    if "POST_RAW" in sections:
        raw = sections["POST_RAW"]
        first, _, rest = raw.partition("\n")
        if first.lower().startswith("content-type:"):
            env["CONTENT_TYPE"] = first.split(":", 1)[1].strip()
            body = rest
        else:
            body = raw
        env["REQUEST_METHOD"] = "POST"
    elif body:
        env["REQUEST_METHOD"] = "POST"
        env["CONTENT_TYPE"] = "application/x-www-form-urlencoded"
    env["CONTENT_LENGTH"] = str(len(body))
    return {"env": env, "body": body}


def run_phpt(path, kernel):
    s = phpt_sections(path.read_text(encoding="utf-8", errors="replace"))
    if "FILE" not in s:
        return "skipped", "no --FILE-- section"
    # run-tests.php runs a test's SKIPIF section and passes the test over
    # when it prints a line beginning with "skip". A section the kernel
    # cannot run says nothing either way, and the test runs, since a test
    # that shows what is missing is worth more than one passed over.
    if "SKIPIF" in s:
        code, out, err = run(kernel, ["--lang", "langs/extras/php.json"], s["SKIPIF"], ".skip.php", beside=path)
        if code == 0 and out.strip().lower().startswith("skip"):
            return "skipped", out.strip()[:80]
    expected = s.get("EXPECT", s.get("EXPECTF", s.get("EXPECTREGEX")))
    if expected is None:
        return "skipped", "no --EXPECT-- section"
    code, out, err = run(kernel, ["--lang", "langs/extras/php.json"], s["FILE"], ".php", web_request(s), beside=path)
    # php-src's own run-tests.php trims both ends before comparing, and
    # a complaint is written with a blank line before it, so the same
    # trim is what the reference expects.
    got = out.strip()
    want = expected.strip()
    # run-tests.php runs a test's CLEAN section afterwards, to take away
    # whatever the test left beside itself. What it prints is nobody's
    # business and whether it worked changes nothing.
    if "CLEAN" in s:
        run(kernel, ["--lang", "langs/extras/php.json"], s["CLEAN"], ".clean.php", beside=path)
    if code == 0:
        ok = (got == want) if "EXPECT" in s else (re.fullmatch(expectf_pattern(want) if "EXPECTF" in s else want, got, re.S) is not None)
        if ok:
            return "pass", ""
        return "differs", "ran, printed something else"
    return "error", reason_of(code, out, err, "PhpError")


# ---------------------------------------------------------------- python

def run_python(path, kernel):
    source = path.read_text(encoding="utf-8", errors="replace")
    code, out, err = run(kernel, [], source, ".py")
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
    kernels = list(KERNELS)
    if "--kernel" in sys.argv:
        kernels = [sys.argv[sys.argv.index("--kernel") + 1]]
    subprocess.run(["cargo", "build", "--release", "--quiet"], cwd=ROOT, check=True)
    php_def = json.loads((ROOT / "langs" / "extras" / "php.json").read_text())
    py_def = json.loads((ROOT / "langs" / "python.json").read_text())
    php_have, py_have = spelled(php_def), spelled(py_def)

    # results[kernel][file] = (status, reason)
    results = {k: {} for k in kernels}
    reasons = {k: {"php": Counter(), "python": Counter()} for k in kernels}
    php_files = sorted((ROOT / "tests" / "php").rglob("*.phpt"))
    py_files = sorted((ROOT / "tests" / "python").glob("*.py"))
    for kernel in kernels:
        for f in php_files:
            status, why = run_phpt(f, kernel)
            results[kernel][f] = (status, why)
            if status in ("error", "differs", "skipped"):
                reasons[kernel]["php"][why] += 1
        for f in py_files:
            status, why = run_python(f, kernel)
            results[kernel][f] = (status, why)
            reasons[kernel]["python"][why] += 1

    php_calls = Counter()
    for f in php_files:
        s = phpt_sections(f.read_text(encoding="utf-8", errors="replace"))
        php_calls.update(calls_in(s.get("FILE", ""), PHP_RESERVED))
    py_calls = Counter()
    for f in py_files:
        py_calls.update(calls_in(f.read_text(encoding="utf-8", errors="replace"), PYTHON_KEYWORDS))

    def totals(kernel, files):
        return Counter(results[kernel][f][0] for f in files)

    def score(c):
        return f"pass {c['pass']}, differs {c['differs']}, error {c['error']}, skipped {c['skipped']}"

    def by_directory(files):
        groups = {}
        for f in files:
            groups.setdefault(f.parent.relative_to(ROOT / "tests").as_posix(), []).append(f)
        return groups

    lead = kernels[0]
    lines = ["# The reference suites against the definitions", "",
             "Generated by `scripts/reference_tests.py`. `tests/php/` is php-src's `tests/lang`,",
             "`tests/basic` and `tests/func`; `tests/python/` is the core-language part of",
             "CPython's `Lib/test`. Each test ran on the " + " and ".join(kernels) + " kernel" + ("s" if len(kernels) > 1 else "") + " with the language's",
             "definition. A test *passes* when it prints what the reference expects, *differs*",
             "when it runs but prints something else, and *errors* when the kernel stops it;",
             "the error's first line, names and numbers folded, is its reason. The reasons,",
             "ranked, are the constructs the definition (or a kernel) does not yet spell, in",
             "the order the reference suites need them. An error is the first one met: a",
             "character the lexer does not know stops a file before any keyword in it is",
             "seen, so the reserved-word and function tables below say what lies behind it.",
             "The reasons are " + lead + "'s; where the kernels disagree on a test, the disagreement",
             "is listed, since the full kernels are meant to behave alike.", ""]
    for name, files in (("PHP", php_files), ("Python", py_files)):
        lines += [f"## {name}: {len(files)} tests", "", "| Suite | Tests | " + " | ".join(kernels) + " |", "|---|---|" + "---|" * len(kernels)]
        for directory, group in by_directory(files).items():
            lines.append(f"| `{directory}` | {len(group)} | " + " | ".join(score(totals(k, group)) for k in kernels) + " |")
        if len(by_directory(files)) > 1:
            lines.append(f"| all | {len(files)} | " + " | ".join(score(totals(k, files)) for k in kernels) + " |")
        lines.append("")
        key = "php" if name == "PHP" else "python"
        lines += ["| Reason | Tests |", "|---|---|"]
        for why, n in reasons[lead][key].most_common(40):
            lines.append("| " + why.replace("|", "\\|") + f" | {n} |")
        lines.append("")
        split = [f for f in files if len({results[k][f][0] for k in kernels}) > 1]
        if split:
            lines += [f"### Kernel disagreements: {len(split)}", "", "| Test | " + " | ".join(kernels) + " |", "|---|" + "---|" * len(kernels)]
            for f in split:
                lines.append(f"| `{f.relative_to(ROOT / 'tests')}` | " + " | ".join(f"{results[k][f][0]}: " + results[k][f][1].replace("|", "\\|") for k in kernels) + " |")
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
    lines += ["## Every test", "", "| Test | " + " | ".join(kernels) + f" | Reason ({lead}) |", "|---|" + "---|" * (len(kernels) + 1)]
    for f in php_files + py_files:
        status, why = results[lead][f]
        lines.append(f"| `{f.relative_to(ROOT / 'tests')}` | " + " | ".join(results[k][f][0] for k in kernels) + " | " + why.replace("|", "\\|") + " |")
    REPORT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    for name, files in (("PHP", php_files), ("Python", py_files)):
        for kernel in kernels:
            for directory, group in by_directory(files).items():
                print(f"{name} {directory} on {kernel}: {len(group)} tests: {score(totals(kernel, group))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
