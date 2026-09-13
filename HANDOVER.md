# Handover — read this and begin

You are picking up `lumen-lang` after the sessions of 2026-09-10,
2026-09-11 and 2026-09-12. This document is the whole briefing. It says where the work
stands, what is waiting on branches, how it is verified, and what went
wrong so that it is not done twice.

**Branch:** `claude/codebase-familiarization-t6vjhi`, cut from `main` at
`7cd5b63` (the merge of batch #486). Everything below the cut is the
integration of the waiting branches, one verification per branch.

---

## 1. Where things stand

**The integration is merged.** Every pull request that was waiting is in
`main` at `73b7ecd`, brought in as #487: twenty-five pull requests in
twenty-two merges, green on all three ways at `a251e5c`. Twenty-four
closed themselves; #480 was closed by hand, its port having kept all the
descriptor work but lost the merge parent that would have made its head
an ancestor. No pull request is open.

The Python reader is done bar three unfinished branches, and the
run-time work has gone a long way: the full `unittest` runs the test
files now, and reports what fails in them.

- **PHP** is unchanged: `pass 399, differs 0, error 0, skipped 23` on both
  full kernels at every verified point, and every change below was
  refused unless that row held.
- **The examples** agree 3006 of 3006 on all six kernels at every
  verified point, on a quiet machine (see §4).
- **0 kernel disagreements and no reference regression**, which had to
  be won: see the two disagreements in §1a.
- **Python, running (stage 2).** The reference row is
  `pass 0, differs 3, error 47` on both kernels at `a251e5c`, against
  `differs 9, error 41` at the old `main`. That is not a step back: the nine
  files that "ran to the end without asserting anything" under the
  minimal `unittest` now run their tests under the full one and stop
  with `Uncaught test run failed` because some assertions fail, which
  the reference runner counts as an error. Their first stops are now in
  the tests themselves rather than in missing modules.
- **Stage 3** (what "pass" means for a `unittest` module: whether the
  harness should run one and count its own results) is still the owner's
  decision and has not been taken. It decides how the Python row is read
  from here on.

### 1a. The six defects the verification found

Four belonged to the merging; two were disagreements between the kernels
that had stood in the line since earlier merges and had been failing the
reference run ever since.

| commit | what it was |
|---|---|
| `fdf00b8` | two stale scratch expectations: `file-iter/15` recorded less progress through CPython's range tests than the kernels now make, and `sorting-semantics/5` expected a list sorted *in place* to print quoted |
| `db1a167` | a compound write on a row lost the cell it was given in microcode7, so `+=` joined instead of extending and `*=` gave the name a new row. The kernel already had the way in; nothing reached it, because the cell-keeping guards tested only the unnumbered spelling of the working while Python's long special-method list wraps every compound write in a numbered landing. The map half of that road had been fixed before and the row half left |
| `427b70b` | a place written into beyond a row now gets its own words, under a new label `ext.system.fault.index.assign`: CPython says `list assignment index out of range` for a store or delete and `list index out of range` for a read, and both kernels said the read's words for all three |
| `950f3bb` | a compound write on a thing standing on a builtin kind now goes through the worth beneath it, so `x += [2]` on a `list` subclass extends where it stands and keeps its class. It closed three narrower splits with it, among them an inversion stack8 refused while microcode7 answered |
| `ce8ca4f` | **a lost merge guard.** `py-sequence-ops` put its new sequence-aware store behind `ext.op.sequence.values`; the merge collapsed both sides into an unconditional `Prim::Restore`. That store reads the place to see whether the thing being written already stands there, and in PHP reading a place that is not there is worth a warning, so `$e["k"][1] = "hello"` on an array with no `k` warned before making it. Five tests of `tests/php/lang` failed on microcode7 alone |
| `a251e5c` | every working with a complex among its values was carried to the complex reckoning, the bit workings included, and the reckoning has no case for them — a number of two parts has no bits — so they fell to its `unready` arm, a NotImplementedError. `~2j` therefore refused as the wrong kind of complaint on microcode7, and `test_unary.py` asks for a TypeError and nothing else |

**How the two disagreements were found, since the job says only that it
failed.** The disagreement detail lives in the run's uploaded artifact,
which these tools cannot fetch. What can be read is the reference job's
log, and in it the per-suite rows:

```
grep -E "^(PHP|Python) .* on (stack8|microcode7):"
```

Each suite is listed once per kernel with its pass/differs/error counts.
A suite whose counts differ between the two kernels *is* the
disagreement, and both of these sat there in plain sight for hours while
the cause was being guessed at from the Python side. Read those rows
first.

**Both suites can then be run locally**, which the notes long said was
impossible. `scripts/reference_tests.py` hardcodes a release binary, but
its `main` is guarded and `BINARY` is a module global, so:

```python
import sys; sys.path.insert(0, "scripts")
import reference_tests as rt
rt.BINARY = rt.ROOT / "target" / "debug" / "lumen-lang"
rt.run_phpt(path, kernel)      # -> (outcome, reason)
rt.run_python(path, kernel)
```

Comparing every test of a suite across both kernels and printing the
ones whose outcome differs named each defect in one run. From there a
`git bisect` with a two-line repro as its test named the commit.

One caution: `run_phpt` writes each test's program beside its `.phpt`,
as php-src's own runner does, leaving `.php` debris. It is ignored now
(`9dea851`), but never commit one: the runner will not overwrite a file
that already exists, so a committed `002.php` would be run in place of
that test's own program from then on.

Brought into the integration branch in this order, each with the
sequence in §3 (and with what its scratch programs then proved missing
ported onto `main`'s kernels afterwards, in each kernel's own words):

| order | branch | PR | commit | notes |
|---|---|---|---|---|
| 1 | `py-test-support-modules` | #463 | `fe0f728` | |
| 2 | `py-stdlib-3` | #482 | `6b357cc` | absorbs `stdlib-2` (#464) and `modules-3` (#468) |
| 3 | `py-stdout-redirect` | #469 | `d5cbfe8` | print through `sys.stdout` |
| 4 | `py-warnings-module` | #478 | `315f0f5` | absorbs `unittest-2` (#472) |
| 5 | `py-complex-type` | #476 | `c46ceb8` | union merge |
| 6 | `py-float-repr-all-kernels` | #479 | `12664fd` | union merge; core label `system.real.render` |
| — | ports for 1–6 | | `674bf76` | verified: PHP row held, 3006/3006, 0 disagreements |
| 7 | `py-slice-object` | #481 | `4156398` + `6b26474` | |
| 8 | `py-hash-eq-identity` | #477 | `aec3e1f` + `39e29d6` + `7f2132f` | |
| 9 | `py-int-methods` | #483 | `3c15848` + `d053343` | |
| 10 | `py-builtin-subclassing` | #474 | `b1bef8a` + `3f2a95a` | the library's own `round` had been shadowing the builtin; see §4 |
| 11 | `py-descriptors` | #480 | `8893805` | |
| 12 | `py-lazy-iterators` | #462 | `4c78b7e` | |
| 13 | `py-dict-semantics` | #459 | `4dbb652` | ported onto `main`'s kernels rather than merged |
| 14 | `py-control-flow-edges` | #460 | `ddd8101` | |
| 15 | `py-reader-tail` | #461 | `0bf373e` | + `1e8919a`, the bare parent word; see §4 |
| 16 | `py-builtins-2` | #466 | `04fd417` | |
| 17 | `py-exceptions-2` | #471 | `28af333` | re-ported on top of `control-flow-edges`, which had done the same work |
| 18 | `py-dunder-2` | #470 | `6c0239e` + `1e8919a` | |
| 19 | `py-range-object` | #457 | `c02ed0b` + `92bbacf` | `main`'s variants kept, range's cases routed through them |
| 20 | `py-sorting-semantics` | #485 | `4739616` | its own last run was red; green here |
| 21 | `py-sequence-ops` | #455 | `41347b0` | |
| 22 | `py-perf` | #473 | `a14aa3f` | head of the line |

All twenty-five pull requests are therefore in the line, in twenty-two
merges: `stdlib-2`, `modules-3` and `unittest-2` are absorbed by the
branches that were cut after them. Of the twenty-five branches
twenty-four are ancestors of `a14aa3f`, so merging the line closes their
pull requests by itself. **`py-descriptors` (#480) is not an ancestor**:
its commit `05de7ef` has a single parent, having lost the merge parent
it was made with, though all of its work is in the line. That one pull
request must be closed by hand.

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

### 1b. The Python run-time work since the merge

The merge is behind us and the work since is a different thing: making
the fifty Python reference test files in `tests/python/` actually run
their own tests and pass them. What follows is what has been learnt, so
that none of it is learnt twice.

**How to count a passing test, and how not to.** `ran - failures -
errors` is not a count of passing methods. `subTest` lets one method
report many failures, so `test_format` prints `Ran 18 tests` with
`failures=5, errors=15`, and the subtraction gives a negative number.
The honest measure is `unittest`'s progress line, one character per
method -- `.` passed, `F` failed, `E` errored, `s` skipped. Check that
its length equals the `Ran N` it reports, then count the dots. Two
figures were circulated from the bad formula before this was caught;
neither should be repeated.

**The library is the near bank, the kernels the far one.** Most files
stop before their first test on a module the library has not got, and
the blockers are chained -- lifting one reveals the next. Since the
merge the library has gained `doctest` (a real one, which finds the
`>>>` examples in docstrings and in a module's `__test__` table, runs
them, and honours the `+ELLIPSIS`, `+NORMALIZE_WHITESPACE`, `+SKIP` and
`+IGNORE_EXCEPTION_DETAIL` directives), `collections.abc`, `types`,
`numbers`, `errno`, `signal`, `shutil`, `dis`, `_string`,
`annotationlib`, `threading`, `ast`, `marshal`, `unittest.mock`,
`_decimal`, `test.typinganndata` and `test.test_math`, along with
`codecs.BOM_UTF8` and `sys.executable`.

`unittest`'s loader also stopped running an abstract base class's tests
a second time through each subclass that inherits from it. That is a
correction, not a loss, though it makes a file's collected count fall:
`test_tuple` went from 60 collected to 38, with the same 18 passing.

**Defects the running found, beyond the six in §1a.** Each was
reproduced against CPython on this machine before it was believed.

- An exception raised inside a context manager's `__enter__` was thrown
  away by stack8 and replaced with the words for a special method that
  gave back nothing usable, so an arm written round the `with` block
  never caught it. Fixed: the raised value is carried out as it is, the
  way every other place that asks a special method already does.
- `print(d)` and `str(d)` of a map give the interpreter's own rendering,
  `[k => 1]`, where Python wants `{'k': 1}`. `repr`, `%s` and an
  f-string are all already right, so only the plain-text path is wrong.
- A failed unpacking assignment is a raw fault that `try`/`except`
  cannot catch, and its message lacks CPython's detail. This is the
  largest single lever found: thirteen of `test_unpack`'s fifteen
  failures.
- A class's `__doc__` cannot be read at all; a function's, a method's
  and a module's all can.
- Found and written down but not yet chased: unpacking a thing that has
  only `__getitem__`; `compile(src, name, 'single')` accepted but not
  honoured; a generator's `type()`, its repr and `gi_running`; a stray
  top-level `break` inside `try`/`finally` ending the program silently
  where CPython refuses the file; `exec` globals not finding a name put
  there by a subscript store, and `iter(genexp) is genexp` false, both
  on microcode7 only.
- Kernel-side blockers on whole files: `test_math` and `test_float` stop
  with `this slice operation is not supported`; `test_long` outruns a
  debug build's patience.

**A text key stored into a row turns it into a map** (`b["k"] = 1`),
silently. That was noticed and not yet fixed; it is written here so it
is not lost.

## 2. What is waiting on branches

Nothing with a pull request. Twenty-five were open when this began, all
twenty-five are in `main`, and none is open (§1). Every one
was green on its own branch (build, 3006 examples, scratch programs, PHP
unchanged) and every one in conflict with `main`, because each was cut
from an older `main` and they touch the same rosters and, worse, the same
kernel dispatch. The bases they were cut from, oldest first, in case a
piece has to be read again in its own setting: `e0e816d`
(`sequence-ops`), `94155bd` (`range-object`, `dict-semantics`,
`control-flow-edges`, `lazy-iterators`), `79eefce` (`test-support-modules`,
`stdlib-2`, `modules-3`, `stdout-redirect`, `unittest-2`,
`warnings-module`, `stdlib-3`), `8d6ad85` (`reader-tail`, `builtins-2`),
`2026e6b` (`dunder-2`), `d7871f3` (`exceptions-2`, `perf`), `25f6848`
(`builtin-subclassing`, `complex-type`, `hash-eq-identity`,
`float-repr-all-kernels`, `descriptors`, `slice-object`, `int-methods`),
`f0503a4` (`sorting-semantics`). Branches cut from `79eefce` applied more
cleanly than the rest.

Three branches still hold unfinished work with no pull request and a red
last run, and are the next work: `py-copy-pickle`, `py-bytes-2`,
`py-file-scope`. Port each from the merged head, one at a time, by the
way described below.

Known and left alone, worth a piece of their own. A row written at a
place named by text is silently turned into a map: `b = [1, 2]` then
`b["k"] = 1` gives `[0 => 1, 1 => 2, k => 1]` on **both** kernels where
CPython raises `TypeError: list indices must be integers or slices, not
str`. That is a wrong answer, not a wrong message, and the deletion path
already refuses such a key — `sequence_subscript_fault` in stack8,
`key_refused` in microcode7 — so only the store path wants the check.
Neither the scratch harness nor CI's `scratch` step caps time or output,
so one program that never stops printing could fill a runner; a guard
wants `timeout` and `ulimit -f` around each run, reporting the runaway
rather than dying (a guarded run over every piece found none today, so
this is a hazard, not a present fault). And a handful of divergences
both kernels share, so none is a disagreement: `[].pop()` printing a
doubled `PythonError: IndexError: pop from empty list` where the words
are not accepted by a label-specific recognizer while `pop index out of
range` is; `del x[0]` on a list subclass; the set methods missing on a
set subclass; `dict(D(...))`; `I(5).numerator` reading as a built-in
method; unary `-` and `+` on an int or float subclass; and CPython's
exact wording for a bad unary operand, `bad operand type for unary ~:
'complex'`, which would want a label of its own. (`py-unicode-text` was merged into `main` as #484
before it was verified and taken back again in `a521d8f`; its work is
not in the line.)

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

**Prefer GitHub Actions to running this sequence by hand.** It runs the
same three ways on every push — `build-and-test`, `reference`, `scratch`
in `.github/workflows/ci.yml` — on runners that are not reclaimed, which
this session's container is (§5), and it builds the release binary
itself. Between them the three jobs are the whole sequence:
`build-and-test` does independence, the `-D warnings` build, the
ported-examples check and all 3006 examples on all six kernels;
`reference` does the PHP row, the kernel-disagreement check and the
regression check; `scratch` runs every scratch program on both full
kernels. So push the line and read the run rather than spending an hour
of a container that has fifty minutes left. To read one: list the runs
for the branch, list the run's jobs to find the failing step, then ask
for the failing job's log — the reply carries a pre-signed URL, and
`curl`-ing that to a file and grepping it locally costs nothing, where
reading the log through the tool costs thousands of lines. In the
scratch log each program is announced by
`=== scratch/<piece>/<n>.py (<kernel>) ===` and a failure shows as a
`diff -u` hunk or an `Expected a successful exit.` line, so the piece at
fault is the nearest such announcement above it; the last line is
`Scratch failures: N`. Keep the local debug build for diagnosis, which
is what it is good for. **A run takes about seventy minutes, and there are
only so many runners, so read the run you want rather than starting
more.** Runs go in parallel up to a concurrency limit and then queue
behind one another. Observed directly: three runs created between 12:21
and 12:47 were all in progress at once, while two created at 13:07 and
13:10 sat queued behind them, waiting for a runner. So an extra push
costs twice over — an hour of machine time repeating what the previous
run was already going to say, and, once the limit is reached, a place in
the queue ahead of the run that matters. There is no permission here to
cancel a superseded run either: the API answers
`403 Resource not accessible by integration`. So gather a round of
fixes, verify them together against the debug build, and push the round
once. What none of that argues for is leaving a commit unpushed:
nothing in this container survives it (§5), and a commit that exists
only here is one reclaim from gone. Commit, push, then wait out the
run you care about rather than pushing again to feel busy. The `reference`
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
  label all six kernels honour — #479 does exactly that for reals. For
  containers, see the next note.
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
- **Container rendering is half done, and the half that exists is a flag
  on the value, not a label.** A row written as a literal prints its
  members unquoted (`[1, hello]`) and a map prints `[k => v]`, as the
  examples require; but a container that a *Python operation* builds
  prints as CPython would. That is carried by the second field of
  `Value::Collection(cell, quoted)` in `stack8` (`value.rs`, read where
  `display` meets a `Collection`) and by the matching flag in
  `microcode7`, set by `.held(true)` in the eight places that build such
  a container — `sorted`, `str.split`, `list.copy`, `dict.copy`, the
  ordering with a key, and `list()` of a view, text, cursor or
  generator, which pass the flag on from what they were given. So
  `print(sorted(w))` quotes and `print(w)` does not, even for the same
  members, and that is deliberate rather than a fault: it buys CPython's
  rendering everywhere the examples cannot see it. Two consequences.
  First, do not "fix" the inconsistency by marking a literal-built row
  quoted: `sort` in place is a core operation all ten languages use, and
  marking its list would break the examples, which is how five earlier
  attempts died. Second, a fixture that expects quotes around the
  members of a list that was *not* built by such an operation is wrong,
  and `sorting-semantics/5` was exactly that. Closing the gap properly
  still wants a **core** label all six kernels honour, as
  `system.real.render` is for reals (#479) — the rendering written into
  stream35, microcode11, microcode4 and stack5 as well, each in its own
  words, and every example that shows a container moved. That is a
  change to the language floor and wants its own branch, not a Python
  piece.
- **A map is a row of pairs searched from the front**, in both full
  kernels, so a large map is quadratic: `scratch/perf/3` takes over two
  hours on microcode7 in a debug build and seconds on a release one.
  That is dictionary work, not `perf`'s, and it is the one place where
  the kernels are not fast. Hashed storage for maps is the fix, and it
  has to be written twice, in each kernel's own words.
- **The library's own bindings must not shadow a builtin word.** The
  Lumen library defines `round(x, decimals)`, and while every global
  name stood in front of a builtin spelled the same, `round(number=5,
  ndigits=2)` complained about a missing `decimals`. Both kernels now
  let only the *program's own* outermost bindings shadow a builtin
  (`program_bound` in `kernels/stack8/src/compile.rs`, `named_in_program`
  in `kernels/microcode7/src/build.rs`). The bare parent word is the
  same problem seen from the other end: `super` outside a class is a
  plain name only when the program has bound it or is about to
  (`1e8919a`).
- **The recursion limit wants more stack than a run is given.** A
  thousand calls deep overflowed the native stack in a debug build
  (`modules-3/3`, `/6`, `stdlib-2/3`, `/6`, exit 134). The whole run is
  now made on a thread with a gigabyte of stack in `src/main.rs`; do not
  take that out.

- **An in-place operator on an instance of a builtin subclass is not
  finished, and the two kernels fail at different points.** Measured at
  `db1a167`: `stack8` refuses `L(list) += [2]` with
  `TypeError: can only concatenate L (not "list") to L` where CPython
  extends and `microcode7` gets it right; `microcode7` refuses
  `D(dict) |= {...}` with `PythonError: unsupported operand type(s)`,
  whose bare `PythonError:` prefix is itself a sign the words are not a
  recognised Python complaint. Each is a defect in one kernel and each
  is therefore a disagreement between them, which the reference run
  counts and refuses. Build on the groundwork `builtin-subclassing`
  (#474) already laid — `kind_class`/`worth_of`/`thing_of_kind` and
  adapter tag 14 in `stack8`, `native_kind`/`underlying`/
  `thing_over_native` and `Wrapped` tag 14 in `microcode7` — rather than
  inventing a second mechanism, and note that `scratch/` covered none of
  it, which is why it survived twenty-two merges.
- **A subagent's report of what it could not fix is a claim to check,
  not a finding.** One reported both kernels refusing two cases as
  pre-existing; measuring both binaries showed its own fix had repaired
  one of them, and that the other was a disagreement between the kernels
  rather than a shared gap. Re-measure before writing anything down.

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

**One worker's cleanup kills another's run.** Several agents share this
container, each in its own worktree. A `pkill -f "lumen-lang --kernel"`
meant to stop one agent's own sweep reaches every other agent's
interpreter too; that is what ended a full scratch run here with no
output at all and an exit of 144, and it cost about an hour before the
cause was known. Stop your own work by its process id, never by a
pattern that matches the binary's name. Write results as each one is
produced, one line per run, so that a run killed from outside still
leaves everything it had already learnt.

**The container dies under a release build, not on a timer.** Four
restarts in one session each killed a run in progress, twice at the
release build, which looked like a fifty-to-sixty-minute reclaim. It is
not: once the release builds were stopped the same container ran for
over two and a half hours without interruption. The restarts correlate
with `stack8`'s release build rather than with the clock — that build is
one compile unit of about an hour over 36,000 lines and is by far the
heaviest thing the container does. Treat the lesson as "do not run a
release build here", not "everything must finish in fifty minutes"; and
note `cargo` cannot resume inside a single compile unit anyway, so a
plain `cargo build --release` never finishes even when nothing kills
it. Two things make it
survivable, and both are environment only, changing no committed file:

```
export CARGO_PROFILE_RELEASE_INCREMENTAL=true
export CARGO_PROFILE_RELEASE_OPT_LEVEL=2
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
```

With those the build comes in pieces and each restart resumes where the
last left off. Run it detached (`nohup … &`) into a log, and read the
log rather than waiting on it. Then run the four checks of §3 one at a
time, each into its own whole log: independence, the scratch suite, the
3006 examples, the reference run. Any one of them fits inside a window;
all of them together do not.

**The writable disk is a fixed allowance, and a worktree per branch
will exhaust it.** Each worktree's own `target/` is about 1.5 GB once
its debug build is warm, so a dozen of them fill the session's share.
When it fills, `cargo` dies with
`failed to write query cache ... No space left on device` and a tool's
own output cannot be written either — which is how one agent's build
was killed without the agent noticing, leaving it waiting on a run that
would never report. `df` is misleading here: the allowance can be spent
while the device looks half empty. Deletes still succeed when writes
fail, so the way out is to remove the `target/` directories of
worktrees whose work is already folded in (`rm -rf <worktree>/target`);
they are pure build output and cost nothing to lose. Better still,
delete a worktree's `target/` as soon as its branch is folded in, and
prefer building a collected fix in the main checkout, whose target is
already warm, to building it again in the worktree it came from.

Two 4-core codespaces exist on the repository (`lumen-a`, `lumen-b`), with
Rust installed under `$HOME/.cargo` and `php` present. Everything is driven
from a shell with `gh codespace ssh -c <name> -- 'bash -lc "…"'`. A
codespace shuts down after four idle hours; ssh-ing to it wakes it. One
release build at a time: a second build started while one runs will be
killed or crawl. Each run of the reference suite takes several minutes
now that the Python files run further.
