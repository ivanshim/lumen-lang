# Handover — read this and begin

You are picking up `lumen-lang` after the sessions of 2026-09-10 and
2026-09-11. This document is the whole briefing. It says where the work
stands, what is waiting on branches, how it is verified, and what went
wrong so that it is not done twice.

**Branch:** `claude/codebase-familiarization-t6vjhi`, cut from `main` at
`7cd5b63` (the merge of batch #486). Everything below the cut is the
integration of the waiting branches, one verification per branch.

---

## 1. Where things stand

The Python reader is done bar the pieces still on branches, and the
run-time work has gone a long way: the full `unittest` runs the test
files now, and reports what fails in them.

- **PHP** is unchanged: `pass 399, differs 0, error 0, skipped 23` on both
  full kernels at every verified point, and every change below was
  refused unless that row held.
- **The examples** agree 3006 of 3006 on all six kernels at every
  verified point, on a quiet machine (see §4).
- **Python, running (stage 2).** The reference row is
  `pass 0, differs 2, error 48` on both kernels at `674bf76`, against
  `differs 9, error 41` at `main`. That is not a step back: the nine
  files that "ran to the end without asserting anything" under the
  minimal `unittest` now run their tests under the full one and stop
  with `Uncaught test run failed` because some assertions fail, which
  the reference runner counts as an error. Their first stops are now in
  the tests themselves rather than in missing modules.
- **Stage 3** (what "pass" means for a `unittest` module: whether the
  harness should run one and count its own results) is still the owner's
  decision and has not been taken. It decides how the Python row is read
  from here on.

Brought into the integration branch, each with the sequence in §3 (and
with what its scratch programs then proved missing ported onto `main`'s
kernels afterwards, in each kernel's own words):

| order | branch | commit | notes |
|---|---|---|---|
| 1 | `py-test-support-modules` | `fe0f728` | |
| 2 | `py-stdlib-3` | `6b357cc` | absorbs `stdlib-2` and `modules-3` |
| 3 | `py-stdout-redirect` | `d5cbfe8` | print through `sys.stdout`; absorbs `stdlib-2` |
| 4 | `py-warnings-module` | `315f0f5` | absorbs `unittest-2` |
| 5 | `py-complex-type` | `c46ceb8` | union merge |
| 6 | `py-float-repr-all-kernels` | `12664fd` | union merge; core label `system.real.render` |
| — | ports for 1–6 | `674bf76` | verified: PHP row held, 3006/3006, 0 disagreements |
| 7 | `py-slice-object` | `6b26474` | verified as this is written; see `verify-merge-7` below |
| 8 | `py-hash-eq-identity` | `39e29d6` + `7f2132f` | scratch green in debug; release verification next |

Ports worth knowing about, since they changed kernel behaviour beyond
the branch that asked for them: `getattr` on a module reads through the
module's cell instead of handing the cell out; a thing's member is
written over rather than through (the write-through, meant for a
module's bindings, had tied a list to itself); the microcode kernel had
lost the `complex` builtin's table entry in the union merge; a class
that annotates nothing carries no `__annotations__`; `ldexp` reaches the
denormals; a real that a math working gave prints in the shortest
rendering on both kernels; what a method of a thing raises inside a
dyadic operation is raised on rather than reported as an invalid
answer; `id` is stable across a list's growth (both kernels hand that
one builtin the cell a collection lives in).

## 2. What is waiting on branches

Twenty-five pull requests were open when this began; eleven of them are
now in the integration branch (marked below) and fourteen still wait.
Every one was green on its own branch
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
| #463 | `py-test-support-modules` | 79eefce | **in the integration branch.** `test.support`, `test.seq_tests`, `test.list_tests`, `sys` extras, `fractions`. |
| #464 | `py-stdlib-2` | 79eefce | **in the integration branch.** `io.StringIO`, `contextlib`, `abc`, `enum`, `dataclasses`, `typing`, a small `re`, `textwrap`, `string`, `time`. |
| #466 | `py-builtins-2` | 8d6ad85 | `globals`, `locals`, `exec`, `eval`, `compile`, `dir`, `__name__`. |
| #468 | `py-modules-3` | 79eefce | **in the integration branch.** `json`, `os` (minimal), `traceback`, `platform`, `locale`, `unicodedata`, guarded C-module stubs. |
| #469 | `py-stdout-redirect` | 79eefce | **in the integration branch.** `print` follows `sys.stdout`; `captured_stdout`. |
| #470 | `py-dunder-2` | 2026e6b | `__index__`, unary and in-place operators, `@`, `__format__`, `__missing__`. |
| #471 | `py-exceptions-2` | d7871f3 | `ExceptionGroup`/`except*`, notes, `sys.exc_info`, `SystemExit`. |
| #472 | `py-unittest-2` | 79eefce | **in the integration branch.** the full `assert*` surface, runner output as CPython's, loader, skips, `subTest`, cleanups. |
| #473 | `py-perf` | d7871f3 | speed trials under `scratch/perf/`; the kernels were already fast. |
| #474 | `py-builtin-subclassing` | 25f6848 | `class X(str)`, `(int)`, `(list)`, `(dict)`, `(tuple)`, `(float)`, `(set)`. |
| #476 | `py-complex-type` | 25f6848 | **in the integration branch.** `complex` as a value. |
| #477 | `py-hash-eq-identity` | 25f6848 | **in the integration branch.** `is`, `id`, `hash`, `==` and the singletons, as CPython. |
| #478 | `py-warnings-module` | 79eefce | **in the integration branch.** `warnings`: warn, filters, `catch_warnings`, warnings as errors. |
| #479 | `py-float-repr-all-kernels` | 25f6848 | **in the integration branch.** CPython's shortest round-trip rendering of reals behind a **core** label `system.real.render`, implemented in all six kernels; the examples agree 3006/3006. |
| #480 | `py-descriptors` | 25f6848 | the `__get__`/`__set__` protocol behind properties and bound methods. |
| #481 | `py-slice-object` | 25f6848 | **in the integration branch.** slice values, `Ellipsis` and tuple keys to `__getitem__`. |
| #482 | `py-stdlib-3` | 79eefce | **in the integration branch.** `bisect`, `heapq`, `statistics`, `string`, `pprint`, `itertools`/`math`/`functools`/`operator` completeness. |
| #483 | `py-int-methods` | 25f6848 | `bit_length`, `to_bytes`/`from_bytes`, `int()` in every base with CPython's errors, `float.hex`. |
| #485 | `py-sorting-semantics` | f0503a4 | stability, `key`, mixed-type errors, `min`/`max`. Its last run was **red**; treat as unfinished. |

Three more branches hold unfinished work with no pull request and a red
last run: `py-copy-pickle`, `py-bytes-2`, `py-unicode-text` (the last was
merged into `main` as #484 before it was verified and taken back again in
`a521d8f`).

### Bringing a branch in

Merge one branch at a time into a worktree cut from the integration
branch's head, resolve, verify with the full sequence, then fast-forward
the integration branch. Two ways of resolving have served:

- **Take `main`'s kernels whole** (`scratchpad/resolve.sh` in the session's
  scratch directory; the idea is what matters): merge the branch, then
  check out the integration head's `kernels/`, `langs/python.json` and
  `langs/README.md` over the merge, keep the branch's module sources,
  scratch programs and fixtures, regenerate the manifest. The branch's
  kernel layer is an older account of what `main` has since done, under
  other names; what its scratch programs then fail on is exactly what
  has to be ported by hand, in each kernel's own words, with each new
  label registered in the five places. This was the way for every
  branch cut from `79eefce` and for `slice-object`, `hash-eq-identity`
  and `int-methods`.
- **Union of the conflict hunks** (`union.py`): ours then theirs in each
  hunk, rosters merged label by label, `BUILTIN_LABELS` count raised,
  the definition JSON merged three ways from the git stages, then hand
  fixes for duplicated match arms. This was the way for `complex-type`
  and `float-repr-all-kernels`, whose kernel work `main` did not have at
  all. It is quick and it is treacherous: the `complex` builtin's entry
  in `kernels/microcode7/src/table.rs` vanished this way and was only
  found when its scratch programs were run on the merged tree.

Whichever way, the branch's scratch programs are the specification; run
them on both full kernels before anything else, and then the whole
scratch suite, since a port that satisfies one piece has more than once
broken another (`slice_value` ordering broke `del`, the member
write-through broke lists held in objects, the `Ellipsis` word was read
out of a quoted string).

Rosters resolve as unions —
`langs/python.json` keys (never one twice; a repeated key parses and the
last wins, silently), the long list lines in `kernels/stack8/src/lang.rs`
and `kernels/microcode7/src/table.rs` (`BUILTIN_LABELS` carries its length
in its type), `langs/README.md` prose kept from both sides and its table
regenerated by `python3 scripts/lang_table.py`. Code resolves by reading
both sides and writing the one function that does what both meant, in
each kernel's own words: `scripts/kernel_independence.py` refuses twelve
identical significant lines between any two kernels.

Order taken and suggested: the branches cut from `79eefce` first (done),
then `25f6848` (`complex-type`, `float-repr`, `slice-object`,
`hash-eq-identity` done; `int-methods` merged and awaiting its port;
`builtin-subclassing`, `descriptors` next), then `94155bd`
(`dict-semantics`, `control-flow-edges`, `lazy-iterators`), then
`reader-tail`, `builtins-2`, `exceptions-2`, `perf`, `dunder-2`, and
#457, #455 and #485 last.

A fixture the branch changed is a signal, not an instruction: where
CPython agrees with the new expectation it is taken (a refusal turned
into a run, the shortest rendering of reals, CPython's own wording for a
truth method's wrong answer); where the old expectation encodes a
documented divergence (`//` truncating, `round` half away from zero) it
stays. `python3` is on the machine: run the fixture's program through it
before deciding.

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

- **The examples are load-sensitive.** `test.sh` times each run; with a
  release build or a debug scratch run going beside it, 22 of the 3006
  came back as "differs" or timed out, and every one passed alone. Run
  step 7 of the sequence on a quiet machine, and rerun it alone before
  believing a failure there.
- **The verification log keeps the tail of each step.** `tail -40` of the
  scratch step showed four of 43 failures; the other 39 were only found
  by running the scratch suite again into a full log. Keep whole logs.
- **A debug build is not a verification.** It builds in minutes where
  the release build of `stack8` takes an hour of one core (the functions
  are huge), so it is the right tool for porting; but its scratch run
  takes hours, it overflows the stack on `unicodedata` (`modules-3/3`
  and `/6`) and on a few deep programs, and a run left going while the
  binary is rebuilt under it reports nonsense. Copy the binary aside
  before a long run.
- **`getattr`, `is`, `id` and `hash` all reach values through cells** in
  both kernels — `Value::Bond`/`Binding`/`Collection` in `stack8`,
  `Value::Shared`/`Mutable` in `microcode7` — and each builtin decides
  for itself whether it wants the cell or the contents. A builtin given
  the cell by mistake ties the program in knots (a list written into its
  own cell overflowed the stack on printing); one given the contents by
  mistake cannot tell two names for one list apart.

## 5. Working with the codespaces and the session machine

The last two sessions ran in a Claude Code remote container with Rust,
`php` and `python3`, worktrees under the session's scratch directory
(`merge-1` … `merge-9`, one per branch, each with its own `target/`), and
the integration checkout at `/home/user/lumen-lang`. That container does
not outlive the session; only what is pushed survives.

Two 4-core codespaces exist on the repository (`lumen-a`, `lumen-b`), with
Rust installed under `$HOME/.cargo` and `php` present. Everything is driven
from a shell with `gh codespace ssh -c <name> -- 'bash -lc "…"'`. A
codespace shuts down after four idle hours; ssh-ing to it wakes it. One
release build at a time: a second build started while one runs will be
killed or crawl. Each run of the reference suite takes several minutes
now that the Python files run further.
