# Handover — read this and begin

You are picking up `lumen-lang` after the session of 2026-09-10. This
document is the whole briefing. It says where the work stands, what is
waiting on branches, how it is verified, and what went wrong so that it is
not done twice.

**Branch:** `main` **Head:** `10eb5ef` (merge of batch #486)

---

## 1. Where things stand

The Python reader is nearly done and the run-time work has begun.

- **PHP** is unchanged: `pass 399, differs 0, error 0, skipped 23` on both
  full kernels, and every change below was refused unless that row held.
- **Python, the reader (stage 1).** Of the 50 CPython files in
  `tests/python/`, 46 read end to end on stack8 and 48 on microcode7 at
  `main`. The four that still stop in the reader — `test_decorators.py`,
  `test_set.py`, `test_with.py`, `test_str.py` — are handled by branches not
  yet in `main` (`reader-tail`, `builtin-subclassing`, below).
- **Python, running (stage 2).** The reference row at `main` is
  `pass 0, differs 9, error 41` on both kernels: nine files now run to the
  end and print something; the rest stop at run time. Their first stops
  are concentrated: `TestCase` and the rest of `unittest` (the `modules`
  branch landed a minimal one; `unittest-2` on a branch has the full
  surface), `sys.maxsize`/`float_info`/`MAX_Py_ssize_t`/`sentinel`/
  `CommonTest` from CPython's own test package (`test-support-modules`),
  `complex`, `Fraction`, `globals`/`exec`.
- **Stage 3** (what "pass" means for a `unittest` module, and whether the
  harness should learn to run one) is still the owner's decision and has
  not been taken.

Landed in `main` during the session, each verified with the sequence in
§3 before merging: decorators, annotations, imports, parameters, slices,
exceptions, strings, blocks, tuples, class, expressions, lexical,
comprehensions, match, numbers, defaults, syntax-modern, builtin-kwargs,
builtins-core, decorators-class, annotated-members, closures,
generators-run, str-methods, str-methods-2, sets, dunder-methods,
numeric-semantics, exceptions-classes, format-spec, modules, bytes,
class-advanced, and the file-driven pieces for scope, syntax, strings,
exceptions, builtin, int, grammar, iter, generators, dict, list, str,
misc, float, decorators, class. Pull requests #408–#486.

## 2. What is waiting on branches

Twenty-five pull requests are open, every one green on its own branch
(build, 3006 examples, scratch programs, PHP unchanged) and every one in
conflict with `main`, because each was cut from an older `main` and they
touch the same rosters and, worse, the same kernel dispatch. They are
listed oldest first with the base they were cut from. `79eefce` is the
head of the `modules` branch, now in `main`; branches cut from it apply
more cleanly than the rest.

| PR | branch | base | carries |
|---|---|---|---|
| #455 | `py-sequence-ops` | e0e816d | `+`, `*`, lexicographic comparison on sequences, tuple immutability errors. Deferred twice: it treats sets as immutable vectors where `main`'s sets are mutable hash storage. |
| #457 | `py-range-object` | 94155bd | `range` as a lazy value. Conflicts with `builtins-core` on `main`: both implement `min`/`max`/`repr`/`reversed`/`tuple`/`set` as different enum variants in both kernels (`Greatest/Least/Represent/Backward` vs `Maximum/Minimum/Repr/Reversed`, `Highest/Lowest/…` vs `Prim::Greatest/Least/…`). Resolve by keeping `main`'s variants and routing range's cases through them. |
| #459 | `py-dict-semantics` | 94155bd | insertion order, views, `\|`, `fromkeys`, `popitem`, mutation-during-iteration. |
| #460 | `py-control-flow-edges` | 94155bd | `finally`/`return`/`break` interplay, `__exit__` arguments, exception chaining, recursion limit. |
| #461 | `py-reader-tail` | 8d6ad85 | the last reader stops (fstring, print, grammar, set, listcomps, funcattrs, opcodes, with, decorators). |
| #462 | `py-lazy-iterators` | 94155bd | iterator protocol; lazy `map`/`filter`/`zip`/`enumerate`. |
| #463 | `py-test-support-modules` | 79eefce | `test.support`, `test.seq_tests`, `test.list_tests`, `sys` extras, `fractions`. |
| #464 | `py-stdlib-2` | 79eefce | `io.StringIO`, `contextlib`, `abc`, `enum`, `dataclasses`, `typing`, a small `re`, `textwrap`, `string`, `time`. |
| #466 | `py-builtins-2` | 8d6ad85 | `globals`, `locals`, `exec`, `eval`, `compile`, `dir`, `__name__`. |
| #468 | `py-modules-3` | 79eefce | `json`, `os` (minimal), `traceback`, `platform`, `locale`, `unicodedata`, guarded C-module stubs. |
| #469 | `py-stdout-redirect` | 79eefce | `print` follows `sys.stdout`; `captured_stdout`. |
| #470 | `py-dunder-2` | 2026e6b | `__index__`, unary and in-place operators, `@`, `__format__`, `__missing__`. |
| #471 | `py-exceptions-2` | d7871f3 | `ExceptionGroup`/`except*`, notes, `sys.exc_info`, `SystemExit`. |
| #472 | `py-unittest-2` | 79eefce | the full `assert*` surface, runner output as CPython's, loader, skips, `subTest`, cleanups. |
| #473 | `py-perf` | d7871f3 | speed trials under `scratch/perf/`; the kernels were already fast. |
| #474 | `py-builtin-subclassing` | 25f6848 | `class X(str)`, `(int)`, `(list)`, `(dict)`, `(tuple)`, `(float)`, `(set)`. |
| #476 | `py-complex-type` | 25f6848 | `complex` as a value. |
| #477 | `py-hash-eq-identity` | 25f6848 | `is`, `id`, `hash`, `==` and the singletons, as CPython. |
| #478 | `py-warnings-module` | 79eefce | `warnings`: warn, filters, `catch_warnings`, warnings as errors. |
| #479 | `py-float-repr-all-kernels` | 25f6848 | CPython's shortest round-trip rendering of reals behind a **core** label `system.real.render`, implemented in all six kernels; the examples agree 3006/3006. |
| #480 | `py-descriptors` | 25f6848 | the `__get__`/`__set__` protocol behind properties and bound methods. |
| #481 | `py-slice-object` | 25f6848 | slice values, `Ellipsis` and tuple keys to `__getitem__`. |
| #482 | `py-stdlib-3` | 79eefce | `bisect`, `heapq`, `statistics`, `string`, `pprint`, `itertools`/`math`/`functools`/`operator` completeness. |
| #483 | `py-int-methods` | 25f6848 | `bit_length`, `to_bytes`/`from_bytes`, `int()` in every base with CPython's errors, `float.hex`. |
| #485 | `py-sorting-semantics` | f0503a4 | stability, `key`, mixed-type errors, `min`/`max`. Its last run was **red**; treat as unfinished. |

Three more branches hold unfinished work with no pull request and a red
last run: `py-copy-pickle`, `py-bytes-2`, `py-unicode-text` (the last was
merged into `main` as #484 before it was verified and taken back again in
`a521d8f`).

### Bringing a branch in

Merge one branch at a time into a clone of `main`, resolve by hand, verify
with the full sequence, then merge. Batching five per verification worked
(PRs #456, #458, #465, #467, #475, #486 each carried five) while the
conflicts were rosters; the branches that remain conflict in kernel code,
so expect one branch per verification. Rosters resolve as unions —
`langs/python.json` keys (never one twice; a repeated key parses and the
last wins, silently), the long list lines in `kernels/stack8/src/lang.rs`
and `kernels/microcode7/src/table.rs` (`BUILTIN_LABELS` carries its length
in its type), `langs/README.md` prose kept from both sides and its table
regenerated by `python3 scripts/lang_table.py`. Code resolves by reading
both sides and writing the one function that does what both meant, in
each kernel's own words: `scripts/kernel_independence.py` refuses twelve
identical significant lines between any two kernels.

Suggested order: the branches cut from `79eefce` (they mostly add module
sources under `langs/lib_python/modules/`), then `25f6848`, then
`94155bd`, then #457, #455 and #485 last.

## 3. The verification sequence

Nothing is merged until all of this has run and each line says what it
should. Run it in a codespace or on a machine with Rust and `php`:

```
RUSTFLAGS="-D warnings" cargo build
python3 scripts/kernel_independence.py           # must say 0 problem(s)
python3 scripts/port_examples.py
python3 scripts/lang_table.py
touch src/main.rs && cargo build --release
for f in scratch/*/*.py; do …run on stack8 and microcode7, compare with the .out or .err beside it… done
TEST_QUIET=1 ./test.sh --lang all                # must be 3006 of 3006
git checkout tests/REPORT.md && python3 scripts/reference_tests.py
grep -c "Kernel disagreements" tests/REPORT.md   # must be 0
git diff tests/REPORT.md | grep -E "^-.*\| pass \| pass \|"   # must print nothing
```

GitHub Actions runs the same three ways on every push (`build-and-test`,
`reference`, `scratch` in `.github/workflows/ci.yml`); the `reference`
job writes the `| all |` and Python rows and the reasons table into the
run summary. `scratch/` holds each piece's programs with the exact output
(`.out`) or first stderr line (`.err`) both full kernels must give; the
`scratch` job runs them all. `test.sh` needs bash 4 or later.

## 4. What went wrong, so it is not done twice

- **The six kernels must print alike.** `test.sh` compares every kernel
  against stream35 on 3006 examples, and the four reference kernels read
  past every `ext.*` label. Five pieces in a row tried to print reals or
  strings-in-containers as CPython does behind an `ext.*` label and broke
  the examples on the two full kernels only. The way through is a **core**
  label all six kernels honour — #479 does exactly that for reals.
  Container rendering (`[1, hello]`, not `[1, 'hello']`) still follows the
  examples.
- **Floor division and rounding stay as the language floor says**:
  `//` truncates and `round` rounds half away from zero for every language,
  the examples depend on it, and CPython's flooring/half-to-even is
  documented as a divergence in `langs/README.md`.
- **A scratch fixture from an earlier piece may encode a refusal a later
  piece rightly removes** (`.err` expecting "not supported"). Updating it
  is right, with the reason written down; a fixture whose old expectation
  CPython would still agree with is a sign the change is wrong.
- **Merging an unverified branch into `main` directly** cost a fix cycle
  each time (#418, #484). Nothing goes into `main` without the sequence.
- **Two pieces implementing the same builtin** (see #457) is the
  integration problem now; assign builtins to one piece at a time.
- **Every roster line is a single long line**; merges take one side whole
  and the other side's labels vanish with no conflict raised.

## 5. Working with the codespaces

Two 4-core codespaces exist on the repository (`lumen-a`, `lumen-b`), with
Rust installed under `$HOME/.cargo` and `php` present. Everything is driven
from a shell with `gh codespace ssh -c <name> -- 'bash -lc "…"'`. A
codespace shuts down after four idle hours; ssh-ing to it wakes it. One
release build at a time: a second build started while one runs will be
killed or crawl. Each run of the reference suite takes several minutes
now that the Python files run further.
