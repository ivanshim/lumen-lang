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

### 1c. Where the Python reference files stand, measured

Taken over all fifty files in `tests/python/` on both full kernels, and
counted from `unittest`'s progress line with its length checked against
the `Ran N` beside it, every line agreeing:

**231 methods pass of 794, across 31 files that run tests. Nineteen run
nothing.** On microcode7 the same sweep gives 195 of 748 across 30
files; the difference is the five disagreements listed below, three of
which are only a debug build running out of patience.

    test_int_literal  6/6     test_slice                 4/11
    test_unary        6/6     test_augassign              3/7
    test_index       33/55    test_pow                    3/7
    test_list        26/71    test_opcodes                3/8
    test_long        25/43    test_decorators            3/16
    test_scope       19/41    test_positional_only_arg   3/28
    test_tuple       18/38    test_format                1/18
    test_bool        16/31    test_funcattrs             1/39
    test_listcomps   16/68    test_unpack                 1/2
    test_range       12/29    test_class                 0/42
    test_with        11/55    test_global                0/20
    test_compare      9/16    test_print                  0/9
    test_syntax      8/109    test_eof                    0/6
    test_dictcomps    8/11    test_contains               0/4
    test_int          7/52    test_setcomps               0/2
    test_keywordonlyarg 5/11  test_genexps                0/1

Running nothing at all: `test_binop`, `test_bigmem`, `test_builtin`,
`test_cmath`, `test_complex`, `test_dict`, `test_enumerate`,
`test_exceptions`, `test_float`, `test_fractions`, `test_fstring`,
`test_generators`, `test_grammar`, `test_iter`, `test_math`, `test_set`,
`test_str`, `test_string_literals`.

Where the kernels do not agree: `test_listcomps` runs its 68 methods on
microcode7 and none at all on stack8, which is the same program as
`scratch/reader-tail/5.py`; `test_syntax` differs by a single method; and
`test_list`, `test_long` and `test_math` each outrun a four-minute
patience on one kernel and finish on the other, which is the debug build
being slow rather than the kernels differing.

The measurement before this one, 139 of 327 across 16 files, was taken
before `doctest`, the library batch and the fixes in §1b. Files that
reach their own tests have roughly doubled since.

### 1c-bis. The count taken again with a longer patience

Taken at `12fabf1` with the per-file patience raised from four minutes
to fifteen, because the shorter one was scoring three files that finish
as though they ran nothing:

    stack8       380 passing of 1,381 methods across 38 files, 12 run nothing
    microcode7   333 passing of 1,239 methods across 37 files, 13 run nothing

Two of the files that run nothing do so by outlasting even fifteen
minutes in a debug build: `test_exceptions` and `test_math`. What they
settle on has to be read from a release run.

The three the shorter patience was losing: `test_dict` finishes in 318
seconds with 48 of 142, `test_list` in 266 with 27 of 71, `test_long` in
243 with 25 of 43. Quoting a total taken with the four-minute cap makes
the number fall while the work rises, which has happened once already.

### 1d. What each file that runs nothing is now waiting on

Measured again after `doctest`, the library batch, the logical operators,
container rendering and `__debug__` all landed. The blockers move as each
one is lifted, so this list is only true of the commit it was taken at;
take it again rather than trusting it.

Waiting on a module the library has not got -- library work, which
rebuilds in seconds:

    test_builtin     143 methods   builtins
    test_dict        105 methods   test.mapping_tests
    test_fstring      95 methods   datetime
    test_grammar      80 methods   inspect
    test_iter         68 methods   builtins
    test_generators                inspect
    test_cmath                     cmath

Waiting on the kernels, which is the harder half:

    test_binop       this class form cannot run yet
    test_enumerate   this class form cannot run yet
    test_complex     this class operation is not supported
    test_float       reversed() is not supported for these values
    test_fractions   a class cannot answer to complex
    test_long        outruns a debug build's patience
    test_set         not yet looked at again
    test_str         not yet looked at again
    test_string_literals, test_math, test_bigmem   not yet looked at again

`test_exceptions`, which is CPython's own file of 118 methods and 2,909
lines, now runs its tests on both kernels rather than stopping at an
import. It is too slow to finish in a debug build, so what it settles on
has to be read from a release run.

Two disagreements between the kernels were noticed while taking this and
have not been chased. `test_exceptions` diverges at its eighth test.
`test_fractions` stops with different words on each kernel -- `Class
RectComplex cannot answer to <built-in function complex>` on stack8,
`Class RectComplex cannot answer to that` on microcode7 -- which is one
defect wearing two faces.

### 1e. What "this class form cannot run yet" actually covers

Four of the files that run nothing stop with the same sentence, so it
reads like one defect. It is not: the sentence is raised from seven
places in `explicit_class` in `kernels/stack8/src/compile.rs`, and the
class body ends up refused for several unrelated reasons. Each was
reduced to a few lines and checked against real `python3` on this
machine. Five separate defects came out of it.

A class body may hold only the member forms the reader knows -- a
method, a `pass`, a plain `name = value`, an annotated one. Anything
else falls to a catch-all that refuses the whole class at the point it
is defined. So all of these, which CPython runs without comment, stop
the program:

    if True: x = 1                for i in (1, 2, 3): pass
    import sys                    from math import pi
    del x                         try: ... except: ...
    x.append(3)                   print("in body")
    while False: pass             x += 1

A class written inside a function is refused outright, whatever its
body, because the reader marks a body unready when it is not the
outermost piece. `def f(): class A: x = 1` never gets as far as `A`.
This is the widest of the five: `test_set` has sixteen such classes,
`test_float` fifteen, `test_cmath` eight, `test_fractions` seven, and
`test_enumerate` and `test_complex` six each.

The lead worth following first on the class inside a function: the only
thing that marks it unready is the line that opens `explicit_class`,
`let mut unready = !self.piece().outermost;`. The rest of the reader
looks as though it would cope -- each member's value goes into a
gensym'd slot which the class is then built from by reading those slots,
and a slot inside a function is an ordinary local. So the cheap first
experiment is to start `unready` at `false` in a worktree and see what
actually breaks; the likely answer is somewhere in how a method or the
class name is bound, since only the outermost bindings have names the
run can work out while it goes.

A class named with a keyword the header does not carry, `class A(object,
metaclass=type)`, is refused, and so is one whose bases are spread from
a sequence, `class A(*bases)`.

A tuple-unpacking assignment in the body, `seq, res = 'abc', [1, 2]`,
is refused. This one alone stops the whole of `test_enumerate`, whose
`EnumerateTestCase` opens with exactly that line.

A chained assignment in the body, `x = y = 3`, is worse than a refusal:
it binds `x` and quietly loses `y`, so the class is built and answers
wrongly later. A refusal is a defect one can see; this one has to be
looked for.

Two bases are fine -- `class A(P, Q)` was checked and works -- so
multiple inheritance is not among these.

### 1f. Two more blockers reduced to a line each

`reversed()` refuses a byte string, and nothing else. Everything else it
is asked about already agrees with CPython -- a list, a tuple, a string,
a range, a dict and its keys, values and items were all checked -- so
the fix is to teach the builtin two more values, not to build anything.
Reversing a byte string gives a list of integers, `[3, 2, 1]`, the same
way walking one does.

    list(reversed(b'\x01\x02\x03'))        CPython [3, 2, 1]   ours refuses
    bytes(reversed(b'\x01\x02\x03'))       CPython b'\x03\x02\x01'
    list(reversed(bytearray(b'\x01\x02')))  CPython [2, 1]

What `reversed()` says when it genuinely cannot walk a value is a
separate defect, left alone for now: CPython raises a catchable
`TypeError: 'int' object is not reversible`, and we raise
`NotImplementedError: reversed() is not supported for these values`, so
a program that catches TypeError around it does not catch ours.

This one refusal stops the whole of `test_float`, which reaches
`LE_DOUBLE_INF = bytes(reversed(BE_DOUBLE_INF))` at line 685 while it is
still importing and never runs a test.

"This class operation is not supported" means a builtin that cannot be
subclassed, and the list of which ones is short. Each of these was
written as `class S(X): pass` and run against real python3:

    subclass fine   int float str list tuple dict set bool object
                    Exception BaseException
    refused         bytes bytearray complex frozenset type

`class ComplexSubclass(complex): pass` at line 38 of `test_complex`
stops that file, and `class X(frozenset)` is the same defect wearing a
different name. So five builtins are missing from work that already
covers eleven.

### 1g. Where the second batch stands, and what is left on it

The branch `claude/codebase-familiarization-t6vjhi` carries six verified
interpreter fixes on top of the merged `12fabf1`, twenty-five commits in
all. Each was reproduced before it was taken and checked again after, on
both full kernels, against real `python3`:

  * `reversed()` walks a byte string, and `float.fromhex` stops doubling
    towards infinity on a zero, which never gets there
  * a class body binds every name a taking-apart or a chain of signs
    names, evaluating the right side once and handing it out left to
    right
  * four places that took the opposite of a whole number and overflowed
    the host; in a build with the checks off the first answered one
    where it should answer zero, so this was a wrong answer waiting as
    well as a death
  * `str()` takes an encoding, and four codecs answer to their names
  * a class may stand on bytes, bytearray, complex, a frozen set or
    enumerate; and the question of what kind a set is stopped answering
    `set` or `frozenset` by chance from run to run
  * a class body may ask a question -- `if`, `elif`, `else` -- before it
    names a member

Four files that ran nothing now run tests, and one that ran already runs
more. Measured on stack8 at `eb04787` against `12fabf1`:

    test_builtin       nothing  ->   17 of 143
    test_complex       nothing  ->   18 of 37
    test_enumerate     nothing  ->   27 of 105
    test_float         nothing  ->   17 of 54
    test_set           nothing  ->  290 of 644
    test_listcomps    16 of 68  ->   18 of 68

and on microcode7 `test_fractions` goes from nothing to 10 of 50.

The sweep finished. The whole-suite figures at `eb04787`, against the
same measuring at `12fabf1`:

    stack8       380 of 1,381 across 38 files  ->  752 of 2,364 across 43
    microcode7   333 of 1,239 across 37 files  ->  719 of 2,272 across 43

Files running nothing fall from twelve to seven on stack8 and from
thirteen to seven on microcode7. Two of the seven outlast fifteen minutes
in a debug build rather than refusing anything, so what they settle on
has to be read from a release run. Nothing measured moved backwards.

WHAT WAS IN THE WAY OF MERGING, AND IS NOT NOW. CI failed on the
`scratch` job alone -- `build-and-test` and `reference` passed every
run -- because three scratch programs were answered differently by the
two kernels, and one fixture cannot record two answers. All three are
mended, each reduced to a few lines against real `python3` first:

  * `scratch/file-iter/14.py`, the enumerate suite: `test_tuple_reuse`
    is decorated `@support.cpython_only` and was being RUN, passing on
    one kernel and failing on the other by accident of allocation. The
    stand-in for that decorator kept the test; it now steps aside with a
    reason, as CPython does on any other implementation, and
    `sys.implementation` exists and says `lumen`. Reference counting and
    the memory-exhaustion tests step aside with it; the IEEE 754 tests
    still run, since the floats are the host's binary64.
  * `scratch/reader-tail/5.py`: `test_no_leakage_to_locals` failed on
    microcode7, which was the kernel in the wrong. What a call could see
    left out the names it reads from the scopes around it; each routine
    now carries those and where they stand.
  * `scratch/file-builtin/28.py`, the builtins suite, `test_exec`: this
    one was already there before the two above landed and had gone
    unnoticed because the file only began running tests in this batch.
    `del d[k]` inside any function or method failed on stack8 for a dict
    and a list alike, and worked at module level. Inside a function the
    shared cell holds the collection at one remove or more (a bond of a
    bond), and the deletion looked through one wrapping only. It now
    follows them all.

A fourth, found while reducing the third and fixed on BOTH kernels: a
module's dictionary reached from inside a function, `del d[k]` with `d`
global and not declared, was never touched; both kernels made a fresh
local of that spelling to delete from. Where names close over, the cell
shared is now the one the read resolved to. Languages whose names do
not close over are untouched, and PHP's `unset` was checked against real
php for the local and the global case.

Found and NOT fixed: `del q["a"]` where `q` is not bound anywhere gives
`TypeError: unsupported operand types` on both kernels where CPython
raises NameError. Consistent across kernels, so it blocks nothing.

The three affected fixtures record the line both kernels now agree on;
no program under `scratch/` or `tests/python/` was edited. Skipping the
CPython-internals tests makes the passing count fall by exactly the
tests that were passing by accident, which is the right way round.

### 1g-bis. The count once the CPython-internals tests step aside

Measured at 78e9c4d, the commit that makes `cpython_only` skip, against
the count at eb04787 just before it:

    stack8       752 of 2,364  ->  743 of 2,366   (43 files, 7 run nothing)
    microcode7   719 of 2,272  ->  706 of 2,274

The fall is the honest one that was promised. Every character that
changed in every progress line was checked: each is a pass, failure or
error turning into a skip, or a failure turning into a pass. Nothing
went from a pass to a failure or an error, and in `test_long`, whose
collected count grew from 43 to 45, no test fails now that did not fail
before. The three files that carry the skips are `test_enumerate` (14
on stack8, 14 on microcode7), `test_syntax` (16 each) and `test_scope`
(3 each), with one or two in `test_list` and `test_long`.

### 1h. The seventh parked raise, and the one fix that covers the rest

The parked-raise shape turned up a seventh time, and this time it was
mended where every instance meets rather than at the caller that forgot.
On stack8 an exception raised inside a comparison dunder escaped every
`except` around the call whenever the comparison ran inside a called
function -- a `def` or a `lambda` -- and ended the run; at module level
the same raise was caught. microcode7 caught all of it. Reduced to:

    def bad(o): raise TypeError("helper")
    class P:
        def __lt__(self, o): return bad(o)
    try:
        (lambda: P() < "a")()
    except TypeError:
        print("caught")        # CPython and microcode7: caught; stack8: the run ended

The comparison path calls the dunder through `special_call`, which
parks what was raised in `self.carried` and returns words in its place,
and the comparison path used `?` on the words without taking the value
back. The try statement (`run_attempt`) now looks for a parked value
before it reads the words, and shows the arms the value. That covers
the comparison path and every other caller that forgot, since the
parked value is by construction the real fault behind the placeholder.

This is what killed `test_fractions` on stack8 at its second test; it
now reaches `Ran 50 tests` and prints the same progress line as
microcode7. A second, library-side defect was in front of it: a fraction
compared with a complex was refused outright; it now answers as CPython
does. What remains: `Fraction(1, 2) < "a"` raises the library's own
wording on both kernels where CPython says `'<' not supported between
instances of 'Fraction' and 'str'` -- consistent across kernels, and
the CPython wording comes from returning NotImplemented from both sides,
which the library does not yet do.

### 1i. A class body runs any statement now

Every statement form a class body used to refuse is taken: a statement
run for its effect, `+=` on a member, `for` and `while` (the loop's
target is a member, as CPython has it; a loop that runs no pass leaves
its names unbound), `import` and `from ... import`, `try`/`except` with
the partial bindings of an arm that raised part-way surviving, `with`,
and `del`. Checked against real `python3` on both kernels with a probe
covering each form, a class re-run in a loop starting clean, and
nothing leaking to module scope.

Two suites that ran nothing reach their tests on both kernels because
of it: `test_grammar` (75 checks, held by one `from ... import` in a
class body at line 32) and `test_cmath` (33, held by a bare statement).
microcode7 read the target of `for j in ():` as the imaginary suffix of
a number rather than as a name and refused the class; settled with it.

What still refuses a class: a `metaclass=` keyword or `*bases` in the
header (`test_binop`, twelve checks, waits on `metaclass=ABCMeta`), and
a class written inside a function, which is on its own branch.

### 1j. Three fixtures CI caught, and a class inside a function

The pull request's first run on CI found three scratch programs whose
records the class-body work had moved past, none of them caught by the
progress-line filter because their `.err` held the refusal text and
not a progress line. `reader-tail/14` exits quietly on both kernels
and on CPython, so its record is an empty `.out`. `syntax-modern/20`
now reaches its annotation, which names a class that was never
defined: CPython ends with `NameError: name 'Missing' is not defined`,
both kernels with the class-operation refusal, so the record moved to
the refusal and the gap is noted here. An annotation naming an
undefined name should raise NameError.

`reader-tail/3` was a real defect: `del abcd[1:2]` inside a test
method refused on microcode7 with "Cannot take a place out of", since
the deletion arm took the cell it was handed to be the collection and
met a mutable standing for the list. The arm now steps through each
mutable or shared wrapping until the collection is in hand, as the
stack8 arm already does. Six deletion shapes checked against
`python3` on both kernels.

A class written inside a function runs on both kernels: each member
lands in a place of the function's own and the class is formed from
those places when the definition runs, a method that reaches a name of
the enclosing function is closed over that frame where the definition
runs, two closures of one definition are two values, and the qualified
name lists only the routines opened inside the enclosing class. A
probe of twelve shapes (closures over enclosing names, a class made
twice from one definition, `super()` inside such a class, a property
and a staticmethod, an exception class, `vars()`, a class inside a
function inside a function) prints the same as CPython on both
kernels. The branch was written before the class-body statement
family landed, and the two met in the class reader on both kernels:
the branch's `reaches_out` and `stands_in_routine` conditions were
carried onto the `gathering()` and `parts()` accessors the family
introduced. `class/8` exits quietly on both kernels and on CPython.

The count at d669238, the pull request's head before the deletion fix,
checked method by method against the sweep at 78e9c4d with no pass
turning into an error or failure: stack8 810 of 2,524 across 50 files
with 4 running nothing (was 743 of 2,366), microcode7 755 of 2,382
with 5 running nothing (was 706 of 2,274). test_grammar and test_cmath
run for the first time; test_listcomps gains five on each kernel.

CI on 401b7b7 found one more fixture the class-body work had moved
past, reader-tail/3, whose record was still the refusal though both
kernels print one progress line; it moves here to the line the
class-in-function work gives, which both kernels print alike.

### 1k. Batch 2 and 3 merged as #489; batch 4 begins

Pull request #489 merged into main at 601a8a2 with every check green
on 0c0f119. The branch restarts on that main. Batch 4 opens with the
one-line method form; verified and waiting to join it are the sequence
protocol (a class with only `__getitem__` walked until IndexError,
and enumerate, zip and map taking any user iterable lazily), once its
one exposure is settled: on microcode7 the members of `dict.items()`
were lists, not tuples, which the branch is mending. A closure written
onto a class as a special method already kept its frame on microcode7
through the class-in-function work, so that branch is dropped.

Gaps found this round and handed out: subscript assignment on a
module name inside a function makes the name local on both kernels
(long-standing, on main too; `callLst[:] = []` costs test_class 23
methods); a class namespace lists attributes before methods rather
than in definition order; generators cannot yield inside try, lack
throw(), and do not carry their return value on StopIteration; the
`metaclass=` header keyword. Recorded, not yet handed out: `*args`
packs into a list rather than a tuple on both kernels; an explicit
`it.__next__()` on a native iterator is an undefined variable; stack8
panics on `del a["k"]` when `__delitem__` is a class-body method
(index out of bounds at compile.rs:619) where microcode7 refuses the
form; `self.d.__setitem__(k, v)` by name is unsupported.

### 1l. Batch 4: four branches folded into #490

Folded after each was checked against python3 on both kernels with
probes of my own beside the agent's: a method written on the line of
its own name (nine shapes); a class namespace in the order the body
bound it (test_class's three definition-order tests pass on both);
the sequence protocol (an object with only `__getitem__` walked until
IndexError, enumerate, zip and map taking any user iterable lazily,
and on microcode7 the pairs of `dict.items()` and `as_integer_ratio`
being tuples at last, plus `__contains__ = None` blocking the fallback
to iteration); and generators that suspend inside try, except, finally
and with, take `throw()` and `close()`, and carry their return value
on StopIteration (test_generators nine to twenty-four on each kernel,
test_enumerate thirty more on each).

Five scratch records move with it, each to a line both kernels print
alike with no method regressing: file-builtin/28, file-iter/12, 13
and 14, reader-tail/4. Every yield-bearing, dict, iterator, generator
and class scratch program (134) was run on the combined tree under
CI's own rule; the Python and PHP example sweeps pass (one example,
sieve.py, timed out under load and agrees on every kernel when run
alone).

Gaps this round records: neither kernel finalises a generator that
nothing refers to, so a `finally` around a suspended yield in an
abandoned generator never runs (generators-run/8 records that line);
a blocked or missing membership says the generic "special method
returned an invalid value" where CPython says `'X' object is not a
container` and `argument of type 'X' is not iterable` (test_contains
asserts the latter text); `close()` follows CPython 3.13 in handing
back what the body returned, which the local python3 3.11 does not
do, so no probe covers it; `*args` packs into a list rather than a
tuple; `it.__next__()` on a native iterator is an undefined variable
(an agent is on it).

### 1m. Batch 4 merged as #490; batch 5 begins

Pull request #490 merged into main at f774a3e with every check green
on 29052e3. The count at eec8af3, the batch's first head and before
the sequence and generator work, checked method by method against
d669238 with no pass turning into an error or failure: stack8 998 of
2,524 across 50 files with 4 running nothing (was 810), microcode7
939 of 2,382 with 5 running nothing (was 755). The sweep of 29052e3
follows here when it lands.

Batch 5 opens with two branches, each checked against python3 on
both kernels with probes of my own beside the agent's: the iteration
dunders called by name on a native value (`it.__next__()`,
`lst.__iter__()`, `s.__len__()`, `t.__getitem__(i)`,
`d.__contains__(k)`), resolved to the same bound method a named
method gets and forwarding to the primitive `next()`, `iter()`,
`len()`, indexing and membership already run; and `*args` gathering
a tuple rather than a list, one line in each kernel's call binding.
Six records move, each to a line both kernels print alike: params/0,
4 and 18 to what python3 prints; reader-tail/4 and 9 gain a pass;
reader-tail/3 turns one failure into an error, since test_funcdef
now passes its starred-call assertion and runs on to an `eval` that
cannot see the enclosing local `f`.

Gaps this round records: `eval` inside a function does not see the
function's locals, and `f(1, x=2, *(3,4), x=5)` is accepted where
CPython refuses the repeated keyword; a missing attribute on a
builtin value says "value has no such method" where CPython names
the type and the attribute; on microcode7 a write through a subscript
on a class attribute (`C.table[k] = v`) fails with "Cannot share
property"; a callable object whose `__call__` is itself an instance
loops forever on both kernels where CPython raises RecursionError
(being guarded on the subscript branch, which it blocks); `hasattr`
answers False for any member of a builtin value; the example sieve.py
takes thirteen seconds alone in a debug build and times out at
thirty under load.

### 1n. Batch 5 merged as #491; batch 6 is four branches

Pull request #491 merged into main at 41e88c7 with every check green.
The count at 29052e3, the head of batch 4 and before batch 5, checked
method by method against eec8af3 with no pass turning into an error or
failure: stack8 1,050 of 2,463 across 50 files (was 998), microcode7
991 of 2,321 (was 939). Two files ran nothing on that head that had
run before, test_generators and test_unpack, and both were hangs the
generator branch below removes: a `throw` into a `yield from` over an
iterator the program wrote, and `x, y, z = thing` where the thing
answers `__getitem__` and lies about its length. The denominators are
smaller for that reason, not because a test went away.

Batch 6 folds four branches, each probed against python3 on both
kernels before merging, ten commits on 41e88c7:

- **A write through a subscript on a global inside a function**
  (`table[k] = v` in a function body, `table` a module global not
  declared `global`) wrote into a fresh local on both kernels; the
  write now reaches the name where it lives, as CPython does since the
  statement binds no name. The same branch guards a callable whose
  `__call__` is itself an instance (`A.__call__ = A(); A()()`), which
  looped without limit; it is refused at the same recursion depth as
  any other call, as CPython's RecursionError is.
- **The two generator hangs above.** A `yield from` over the program's
  own iterator is stepped a member at a time, so a `throw` or `close`
  reaches the inner generator and a `StopIteration` from it ends the
  walk; and unpacking asks a `__getitem__`-only thing for one member at
  a time until IndexError, rather than gathering it first through a
  `__len__` it may not honour.
- **A write within a class's own value** (`C.table[k] = v` on a class
  attribute, and `self.rows[i] = v` where `rows` is shared by the
  class) reached a copy on microcode7 ("Cannot share property") and
  lost the write; it now reaches the value the class keeps.
- **`isinstance` and `issubclass` ask the metaclass first**, so
  `ABCMeta.__instancecheck__` and `__subclasscheck__` run and
  `register()` on an abstract class answers as CPython does;
  `langs/lib_python/modules/abc.py` grows the registry and the two
  hooks, and `collections/abc.py` and `numbers.py` lean on it rather
  than on lists of their own.

No record moves with those four: the 190 scratch programs the batch
touches print what their fixtures hold on both kernels, the sixteen
unittest fixtures that have moved since d669238 are unchanged, and the
Python and PHP example sweeps pass alone what they timed out on under
load (sieve.py, and four PHP programs on stack8, all with empty output
at thirty seconds and their expected output when run alone).

Three more folds follow on the same pull request. The nested-generator
subscript write above is fixed on microcode7, and with it a routine
named on the right of `and` or `or` (`hard and advance_hard or
advance`) is handed back as a value rather than run, so the `conjoin`
doctest of test_generators finds its tours on both kernels. `eval` and
`exec` inside a function see the function's locals: the caller's
frame is set aside for the text, compiled and run against a detached
copy, so a write inside the text never reaches the routine (CPython
3.13 and later behave the same, and `locals()` after such an `exec`
does not show the text's names); a repeated keyword in a call is
refused while reading, a keyword repeating a positional through `**`
raises CPython's TypeError, a duplicate parameter in a `def` is
named, and a SyntaxError from evaluated text carries its place. And
`del D.table["z"]` through a class attribute, or `del
type(self).items[0]` through a call's result, reaches the value the
class keeps on stack8 as it already did on microcode7. Two records
move with the eval work, both kernels alike and no pass lost:
reader-tail/1 gains two passes (`...EEEEEEEEEEE.EE..EE.EEFE.E..FE...E.EEFF..FE.EF.FFEE......EEEE....E.EEE.E.....EF.EF..FFFF..`)
and reader-tail/9 one (`.....E..EF..E..E`). CI then caught two
more that the local subset had not covered, both wordings now
CPython's own: params/12 (`f(**{"x": 1}, **{"x": 2})`) says
`__main__.f() got multiple values for keyword argument 'x'`, and
params/17 (`def f(a, a)`) says `duplicate argument 'a' in function
definition`. A lesson for the local check: the subset must include
every directory whose programs touch the changed behaviour, and
`scratch/params` holds the call-binding programs.

Gaps this round records, beyond §1m's: `del a["k"]` panics stack8
when the class body defines `__delitem__` (compile.rs:619, not the
class-attribute case above); `lst.__setitem__(i, v)` and the other
operator dunders called by name on a builtin value are not found on
either kernel (a branch is under way); a TypeError for a keyword
repeating a positional lacks CPython's `f()` prefix; a class body has
no namespace of its own in either kernel, so `locals()` there answers
the module's names and `eval("v + 1")` in a class body cannot see
`v`; test_funcdef now runs on to the library's stub `NotImplementedError:
syntax checks need a compile builtin`; and unreferenced generators are
never finalised, so a `finally:` in a generator that is dropped
mid-walk does not run (generators-run/8 records this as a known
divergence).

### 1o. Batch 6 merged as #492; batch 7 begins

Pull request #492 merged into main at a6ec218 with every check green
on cbd0553. Its first run on 1f0ea37 was red on the scratch job for
the two params records §1n describes, and nothing else. The count at
cbd0553, checked method by method against 29052e3 with no pass
turning into an error or failure: stack8 1,114 of 2,536 across 50
files with 3 running nothing (was 1,050 of 2,463), microcode7 1,047 of
2,394 with 4 running nothing (was 991 of 2,321). Eleven files that
ran nothing at 29052e3 now run on stack8 (test_binop, test_builtin,
test_cmath, test_complex, test_dict, test_enumerate, test_float,
test_fractions, test_grammar, test_list, test_set) and ten on
microcode7 (the same list without test_dict, plus test_long), which
is the metaclass, generator and subscript work of batches 5 and 6
letting their imports and class bodies through; test_syntax shows
fewer passes only because `@cpython_only` now skips what it should.

Batch 7 opens with the Fraction formatting work: `Fraction.__format__`
after CPython 3.14's fractions module, in Python under
`langs/lib_python/modules/fractions.py` alone. The library's `re`
has no groups of the `(?P<name>...)` kind, so the two specification
grammars are read by hand rather than by CPython's matchers, with
the same rules (a lone `0` is a width, a `0` before a digit is the
zero-pad flag; width and precision are `[0-9]*`). Rounding uses
`divmod`, which floors as CPython does, never `//` or `%`, which
truncate in this project by design (§4); parity is `abs(n) % 2`.
Five methods of test_fractions pass on both kernels
(`..F..E.EEEEFEEFEFEE.EEFFEE.F..EEEE.E.EEE......EEFE`, from
`...EEEEEEE.EEEFE` at the end), and a probe of a hundred
specifications agrees with CPython's float formatting wherever the
value is a float.

Pull request #493 merged that fold into main at 2e5a006, green on
every job. Batch 7 continues on the next pull request with two more
folds, each probed against python3 on both kernels:

- **A builtin value answers the operator dunders by name.**
  `lst.__setitem__(i, v)`, `__delitem__`, `__add__`, `__eq__`,
  `__iadd__` and the rest were undefined names; they now resolve to
  the bound member a named method gets and forward to the sign
  already run. Two cell faults came out with it: a container asked
  for a member lost the cell its names share (stack8 unwrapped the
  bond in `Grab`, microcode7 read the value out of `Shared` in
  `prim`), so a writing member wrote into a copy; and stack8's
  `__setitem__` stored a bond inside a cell, after which `__iadd__`
  on the same list fell through to a pure `+` whose answer was
  dropped. Both are fixed; a bare `a.__iadd__(x)`, `__imul__`,
  `__ior__`, `__iand__`, `__isub__`, `__ixor__` mutate the name on
  both kernels, and `a.__setitem__(0, b)` keeps `b` shared.
  test_list gains four passes and turns one error into a failure
  (test_setitem now runs and asserts the TypeError a list should
  raise for a text key, which neither kernel raises).
- **`del a["k"]` reaches the `__delitem__` the class wrote** on
  stack8 (it reached for a name inside the method instead), and
  `issubclass`/`isinstance` answer for every builtin kind
  (`issubclass(int, range)` is False, not a refusal), with CPython's
  TypeError for a non-class argument. file-builtin/28 gains a pass.

Gaps this fold records: `'{}'.format(obj)` and `'{:spec}'.format(obj)`
do not reach `__format__` on either kernel (the method goes through a
writer that cannot call back into the interpreter: `Writer::template`
in stack8's engine.rs, `fill_fields` in microcode7's members.rs),
while `format(obj, spec)` and f-strings do; `'%s' % obj` ignores
`__str__` and the two kernels word the failure differently;
test_fractions' `test_float_format_testfile` wants `open` and a data
file the tree does not hold.

### 1p. Batch 7 merged as #493 and #494; batch 8 begins

Pull request #494 merged into main at 6348f05 with every job green on
c9766bd; #493 went in before it at 2e5a006. The Lumen example sweep,
still running when c9766bd was pushed, finished with two programs
timed out under load (fibonacci_iterative and sieve) that print the
same output on stream35, stack8 and microcode7 when run alone.

The count at 6348f05, taken with a copied binary over the fifty
reference files: stack8 1,127 pass of 2,536 ran, microcode7 1,059 of
2,394, no pass lost against cbd0553. test_fractions rose from 12 to 17
on both kernels with `Fraction.__format__`, test_list by four, and
test_bool, test_builtin and test_index by one or two each.

Batch 8 opens with the wording of a missing member. A missing
attribute on a builtin value said "value has no such method"; it now
says what CPython says, naming the kind and the member (`'tuple'
object has no attribute 'append'`), with the class form (`type object
'C' has no attribute 'zz'`), the module form (`module 'math' has no
attribute 'zz'`), and `hasattr` answering for the members a builtin
value does have. Two records move, both kernels alike:
class-advanced/2.out takes CPython 3.13's wording for a write to a
`__slots__` instance (`'C' object has no attribute 'z' and no __dict__
for setting new attributes`), and reader-tail/4.err gains nothing but
prints the same line with the new wording beneath it. The branch had
to take the operator-dunder fold into itself first, since both rewrote
the member road in stack8's engine.rs; the merge kept both (the
dunder probes stay identical on both kernels).

Gaps this round records: `hasattr(int, "real")` answers False, since
a builtin kind's own members are not modelled; `frozenset` and `set`
share one word at the value level, so `isinstance(frozenset([1]),
set)` is True; `type(slice(1, 3))` stops with "unknown value type";
`isinstance` accepts a list of kinds where CPython wants a tuple;
`testHashComparisonOfMethods` in test_class is an error on stack8
and a failure on microcode7.

Three more folds followed the attr-wording one. A list written at a
key that names no place stays a list (`b["k"] = 1` raises `list
indices must be integers or slices, not str` instead of turning the
row into a map), and arithmetic between kinds it means nothing for is
refused in Python under a new switch, `ext.op.arithmetic.strict`, that
only the Python definition turns on: `"a" + 1` says `can only
concatenate str (not "int") to str`, `1 + "a"` names the sign and both
kinds, `None + 1` is refused, and a compound write names its compound
sign. PHP and Lumen keep the shared arithmetic. Two sequence-ops
records (24 and 8) that joined text to a number in the open catch the
refusal where it stands; the porter writes `str()` around a side it
knows to be no line when it spells a Lumen join with the adding sign
for Python, so `examples/python/constructs/string_operations.py` says
what it said before. test_list gains test_setitem and
test_setitem_error on both kernels.

Text formatted by method or by mark reaches an object's own words:
`'{}'.format(obj)` and `'{:spec}'.format(obj)` reach `__format__`,
`'%s' % obj` reaches `__str__` (text on the left of the remainder sign
fills its own marks before either side is asked for a method, except
for a subclass of text hooking `__rmod__`, which keeps its road), and
`"{}".format` read without being called is a bound value like any
other member. A text written as a representation in a field is quoted
by the same hand as `repr()`, so `'%r' % '\u0378'` prints instead of
refusing; a caught `'{x}'.format()` says `KeyError: 'x'`; an opening
brace that ends the text says `Single '{' encountered in format string`
under `ext.text.format.brace.single`. format-spec/20 and 24 move from a
refusal to the output CPython prints. test_str gains one method;
test_class's testMisc goes from an error to a failure (it now reaches
a real assertion about `__eq__` operand order).

Gaps those two rounds record: `b"ab"["k"]`, `bytearray(b"ab")["k"] =
99` and `del bytearray(...)["k"]` give generic wording where CPython
names byte or bytearray indices; `{"a": 1} + {"b": 2}` says
`unsupported operand types` without the kinds; `SubInt(1) + "a"` names
`'int'` rather than the subclass; `b"a" + "b"` and `"a" + b"b"` refuse
with a NotImplementedError where CPython says `can't concat str to
bytes` / `can only concatenate str (not "bytes") to str`; a complex
number cannot be formatted to a spec and `format(3, "n")` is refused,
which is what stops three of test_format's five failing methods, while
test_str_format needs some twenty CPython 3.14 `%`-error wordings the
definition does not hold; a KeyError built from a message keeps its
key double-quoted when caught (`{}.pop("x")` → `KeyError: "'x'"`), and
`s.remove(5)` on a set says `KeyError: '5'` for an int key; `ascii()`
is not defined; `'{:>5}'.format([1, 2])` says "this format cannot be
represented" where CPython raises `TypeError: unsupported format string
passed to list.__format__`.

The fourth fold is membership, hashing and the directory of a builtin.
A membership question the kernels could not answer now says why, in
CPython's words, under three new labels (`ext.op.in.text`,
`ext.op.in.uncontained`, `ext.op.in.declined`): `1 in "abc"` says `'in
<string>' requires string as left operand, not int`, `1 in 5` says
`argument of type 'int' is not a container or iterable` (the 3.14
phrase test_contains asserts; 3.11 said "is not iterable"), and a class
setting `__contains__ = None` says `'C' object is not a container`. A
span of numbers has a hash folded from its count, start and stride, so
`range(3)` and `range(0, 3, 1)` share a place in a set or map. `dir()`
of a builtin value, or of its kind, names the members that kind answers
to, from the same per-kind tables the members are resolved through.
There is now one routine per kernel for the address a value takes among
a set's members — a builtin value by its worth, a thing by the hash and
equality its class gives it — on every way in (braces, `set()`,
`frozenset()`, a comprehension, add, update, remove, discard, `in`), so
two equal things share one place and sets gathered in different orders
agree; a member taken out of a set is the thing itself, not the pair it
was kept as; ±inf is one key and NaN keys by identity. A set method
kept as a bound value and called later, as `assertRaises` calls it,
opens its arguments as any other callee (it was handed the spread
marker itself, for any value), and a missing set member reads
`KeyError: 5`, not `KeyError: '5'`. The library's `deque` gains
`__len__`, `__iter__`, `__reversed__`, `__getitem__`, `__contains__`,
`__eq__` and `__repr__`, and `_NeverEqual` in the test-support stub
gains `__ne__` and `__hash__`. Records moved: file-iter/13 (test_contains)
to an empty .out — all four methods pass; file-iter/15 (test_range)
gains one method; expressions/9, file-exceptions/23 and file-iter/17
take the new membership wording; reader-tail/4 (test_set) ends with 33
methods gained against 6348f05 and none lost, test_badcmp on TestSet
and TestFrozenSet among them. test_dict gains three on stack8 and
test_class one on both kernels.

Gaps that fold records: `{slice(1, 2)}` refuses to hash although
`hash(slice(1, 2))` works (3.12+ allows both); the repr of a set holding
objects shows `{<object Ok>}` instead of calling `__repr__`, as lists
and maps do; `s in [s]` for an instance of a class standing on `set`
answers False on stack8 and True on microcode7 (CPython: True); a
bound `bytearray(b'ab').decode` called through a spread still refuses;
fifteen names, the set methods and `bytes.decode` among them, refuse a
bare unbound read though they work when called.

### 1q. Batch 8 merged as #495; batch 9 begins

Pull request #495 merged into main at 83ae1ad with every job green on
bb4ad6e. The count at bb4ad6e, taken with a copied binary over the
fifty reference files: stack8 1,211 pass of 2,536 ran, microcode7
1,155 of 2,394 (from 1,127 and 1,059 at 6348f05). test_set rose from
362 to 395 on both kernels, test_str from 27 to 59 on stack8 and 73 on
microcode7, test_with by six, test_dict by three on stack8, and
test_bool, test_class, test_contains, test_float, test_list,
test_long, test_range and test_slice by one or two each. Two passes
were LOST and are batch 9's first duty: test_compare's test_bytes on
both kernels, because the strict-kinds check now refuses `b"a" <
bytearray(b"b")`, which CPython allows (bytes and bytearray are one
family for ordering); and test_generators' test_pickle on stack8
alone, because `pickle.dumps(gen())` now aborts the run with `'start'
is not a function` where it used to raise a catchable PicklingError
(CPython: `TypeError: cannot pickle 'generator' object`). The lesson:
the count sweep is the only check that sees a pass lost in a file no
fixture covers, so read it before the next push, not after.

Batch 9 opens with bytes. A row of bytes answers to all 42 non-dunder
members of its kind, on bytes and bytearray alike and through
getattr, where members sharing a name with a global builtin word
(`hex`, `count`, `index`, `rsplit`, `endswith`, `rfind`, `lstrip`,
`rstrip`, the `is*` family, `fromhex`) used to stop with a missing
member or reach the text builtin of that name; the faults are worded
as CPython words them (`byte indices must be integers or slices, not
str`, `bytearray indices ...`, `can't concat str to bytes`, `can only
concatenate str (not "bytes") to str`, `unsupported operand type(s)
for +: 'dict' and 'dict'`, a subclass named by its own class, a
repetition naming the side that is no sequence); `del ba[i]` and `del
ba[i:j]` shorten a bytearray. Five labels join the definition. Gaps
that fold records: bytes `%`-formatting is not implemented;
bytearray's own mutators (`extend`, `insert`, `pop`, `remove`,
`clear`, `reverse`, `copy`, `__iadd__`) do not resolve; `sum([{1:
2}])` says `sum needs numbers`; a bound builtin method's repr is
`<member wrapper>`; stack8 cannot `del d["k"][1]` inside a function.

Asking a value its kind answers for every kind a program can hold:
`type(slice(1, 3))`, `type(b"b").__name__` and `type(f).__name__` used
to stop the run; both kernels now give CPython's name for the builtin
kinds, `NoneType`, `function`, `builtin_function_or_method`, `method`,
`type`, `module`, `generator`, `complex`, `ellipsis`,
`NotImplementedType` and the iterator kinds (`list_iterator`,
`dict_keyiterator`, `list_reverseiterator`, `set_iterator`,
`range_iterator`, and `enumerate`, `zip`, `map`, `filter`, `reversed`
as the builtins spell them), and two readings of one plain kind are the
selfsame value under `is`. A call that fills a place twice names the
routine it was meant for (`f() got multiple values for argument 'a'`,
with the class, method and nested forms) under
`ext.syntax.call.amiss.positional`. Records: params/9 takes the new
wording; file-iter/14 gains test_range_optimization; file-builtin/28
turns one error into a failure; reader-tail/4, re-measured on the
folded tree, gains eight against main with none lost. Gaps: `reversed(d)`
on a dict answers `generator` where CPython names
dict_reverse*iterator; `repr(type(int))` prints `<built-in function
int>` and `repr(type(None))` prints `NULL`; `type(f).__qualname__` and
`__module__` raise; `type({}.keys()).__name__` is `list`;
scratch/file-dict/11 records `multiple values for argument 'x'` for
`dict(**{"x": 1}, **{"x": 2})` where CPython says `dict() got multiple
values for keyword argument 'x'`.

The two passes batch 8 lost are back. A writable row of bytes had
gained its own kind word, so the check that two values stand in some
order no longer saw bytes and bytearray as one family; each kernel now
has an ordering family that folds the two, and test_compare's
test_bytes passes again on both kernels. A member a builtin value has
not got was glanced for as a name nearby and called with the value in
front, a pipe CPython has no notion of; on stack8, where a module's
function locals leak into the module's namespace (`hasattr(marshal,
"start")` is True there and False on microcode7, a pre-existing
divergence), the pickler's own blank local `start` was found and
invoked, ending the run outside any except. A definition that words
the missing-member complaint now raises it and nothing else, so
`pickle.dumps(gen())` is a catchable PicklingError again (CPython's
`TypeError: cannot pickle 'generator' object` would mean rewording the
library's own marshal refusal) and `(1).x` with an `x` bound nearby is
an AttributeError. Gaps: `type(gen())`, `type(f)`, `type(module)` and
`type(C)` as values stop with "unknown value type" on both kernels
(their `__name__` is right); `dir(gen())` and `dir(fn)` are nearly
empty; stack8 evaluates a call's arguments before raising the
missing-member fault, microcode7 after.

The batch-9 count, taken at c862af4 with the sweep binary copied
aside, stands at stack8 1,214 of 2,536 and microcode7 1,157 of 2,394,
against 1,211 and 1,155 at bb4ad6e; no file lost a pass, test_compare's
first position and test_generators' fortieth on stack8 are dots
again, and test_enumerate gained one on both kernels. The scratch
check on the same tree found one record made by the old missing-member
pipe: scratch/file-list/10 held `[1, 2]` for a `.seq` read off a
number that CPython refuses, and now records the AttributeError all
three agree on.

### 1r. Batch 9 merged as #496; batch 10 folds the attribute road and the repr work

Pull request #496 merged into main at 65a14a1 with both runs green on
0cb8dfb. Batch 10 folds two branches. The attribute road: `del (a).b`,
`del (a)[0]` and `del (1).x` were refused as syntax because a deletion
target had to begin with a bare name; both kernels now look past the
matching close bracket, and where a member or an index follows, what
stands inside is the thing a place is reached through. A class that
names its slots and holds a plain class attribute under another name
refuses a write to it as CPython does (`'B' object attribute 'y' is
read-only`, under ext.stmt.class.detail.attribute.readonly), while a
plain class and a subclass naming no slots keep their namespace. A
builtin kind carries what its own values answer to, so
`list.__setitem__(a, 0, 9)`, `list.append(a, 1)`, `dict.get(d, "k")`,
`str.upper("a")`, `int.__add__(1, 2)` and a bound `m = list.append`
all work, and `hasattr(int, "real")`, `getattr(list, "append")` and
`"real" in dir(int)` answer truthfully. test_class gains
testObjectAttributeAccessErrorMessages on both kernels; reader-tail/4
(test_set) gains the thirty-three mutation tests that hand `set.union`
round as a value. Gaps: `hasattr(range, "start")` and `hasattr(bytes,
"hex")` are False on the kinds; `exec("a[0] = 7", globals())` refuses
on microcode7; `type(list.append).__name__`, arity and receiver
wording wait for a descriptor model.

The repr work: `str(e)` for a KeyError raised on a tuple key stopped
with "this exception operation cannot run yet"; `ascii()` was an
undefined name; `repr([].append)` was `<built-in method>` and
`repr(dict.get)` ended the run with a bare "Unsupported string
format"; `repr(int)` was `<built-in function int>` and
`repr(type(None))` was `NULL`; `"%r" % (lambda: 1)` ended the run the
same way. Now, on both kernels, a caught KeyError's text is the key's
own repr whatever its kind and its args are a fixed row; `ascii()`
writes a repr with every character outside ASCII escaped as \x, \u or
\U (ext.builtin.ascii); a bound builtin method says whose method it is
with its address, one read off the kind itself says `<method 'get' of
'dict' objects>` (the attribute road's loose member and the repr
fold's method writing met here: folded together the loose member
printed `<member wrapper>` until 43e7e74), a builtin kind is written
`<class 'int'>`, a `%r` or `{!r}` field writes what a quoting writes
instead of refusing, and a plain function is written `<function f at
0x1>` with its qualified name (`C.m`, `outer.<locals>.inner`) or
`<function <lambda> at 0x1>`, the whole-value address being the one
figure a record can hold, as `<map object at 0x1>` already is.
reader-tail/1 and reader-tail/9 gain one each, reader-tail/4 gains
eight over the attribute road's line (436 of 644 pass, none lost
against either parent), and expressions/13 records the line CPython
prints. The two folds did not build together at first: the attribute
road matched microcode7's builtin word as a one-field value where the
repr fold had given it a primitive beside the word (ca2bd85). Gaps: a
lambda's `__name__` answers `{closure}`, the compiler's one name for
every anonymous routine in every language; a bound method still
prints `<function(a, b)>`; `dir(f)`, `__qualname__`, `type(gen())` and
`type(f)` as values are untouched; `ascii("\ud800")` cannot be
represented, the text kind holding UTF-8; a user class's instance
prints `<object C>`.

The frozenset branch (fix/frozenset-kind) reported complete in this
batch's time, with frozenset a kind of its own in both value models
(ext.builtin.frozenset), its probe identical to python3, and test_set
gaining fifteen tests, but it is held back: `hash(frozenset(...))`
answering lets test_hash_effectiveness run, and that walks every
subset of up to seventeen members twice, which took the debug binary
2014 s on stack8 and 3115 s on microcode7 for reader-tail/4 against
70 s and 180 s before, over every 900 s cap this project applies
(fixture checker, scratch checker, count sweep, and CI's release cap).
A performance pass on that branch precedes its fold.

The batch-10 count, taken at e1c18cc with the sweep binary copied
aside, stands at stack8 1,261 of 2,536 and microcode7 1,203 of 2,394,
against 1,214 and 1,157 at c862af4; no file lost a pass. test_set
rose from 395 to 436 on both kernels, and test_class, test_complex,
test_decorators, test_float and test_fstring gained one each on both,
test_dict one on stack8. The scratch check over the 380-program list
found nothing to move beyond the four records named above.

### 1s. Batch 10 merged as #497; batch 11 folds format() and the writable row of bytes

Pull request #497 merged into main at c9da92d with both runs green on
a8e700c. Batch 11 folds two branches. format() and `__format__`:
`format(1+2j, "")` ended the run with a bare "Unsupported string
format" that no try could catch, and so did `"%r" % (lambda: 1)`;
`format(3, "n")` was refused; a list, a dict, None or a plain object
given any spec, and a text given `d`, answered nothing CPython would.
Now, on both kernels, a complex number is written by `format` and its
own `__format__` with the spec's precision, kind, sign and width
(`'1.00+2.00j'`, `'+1j'`, `'    (1+2j)'`), a zero-padded or
alignment-marked complex spec is refused in CPython's words
(ext.text.format.complex.zero, ext.text.format.complex.align), the
`n` presentation is `d` or `g` in the C locale, an empty spec on any
value is its str(), a non-empty spec on a list, dict, NoneType or
object says `unsupported format string passed to list.__format__`,
and a code that does not fit the kind says `Unknown format code 'd'
for object of type 'str'`. test_complex gains test_format, test_format
gains test_locale, test_negative_zero and test_non_ascii, test_fstring
gains two under test_errors and reader-tail/1 records one more,
test_float gains FormatTestCase.test_format; format-spec/13 prints its
`1`. The branch had also stopped the `%r` abort on a lambda by its own
road, a plain display of the value; the fold keeps batch 10's quoting
road (with the `%a` escaping) in both kernels' formatting.rs where the
two met.

The writable row of bytes: `extend`, `insert`, `pop`, `remove`,
`clear`, `reverse`, `copy`, `__iadd__` and `__delitem__` were missing
on a bytearray, `b += ...` did not keep the row's identity, and every
`%` on bytes stopped with "this bytes operation is not supported".
Now, on both kernels, a bytearray answers to each of those methods
through getattr as well, with CPython's faults (`byte must be in
range(0, 256)`, `'str' object cannot be interpreted as an integer`,
`value not found in bytearray`, `pop index out of range`, `pop from
empty bytearray`), `+=` and `*=` change the row in place, and bytes or
a bytearray on the left of `%` fill %d, %s, %r, %a, %b, %c, %x, %o,
%%, width, precision and a keyed map through the same spec reader the
text kind uses (ext.op.rem.format.byte), refusing a text for %b and a
non-number for %d in CPython's words. test_format gains one method on
both kernels.

Waiting for batch 12, verified in their worktrees: the map store
(fix/dict-microcode7): a map was a row of pairs searched from the
front in both full kernels, so test_dict never got past its
fourteenth test on microcode7 within the 900 s cap and
scratch/perf/3 timed out on both kernels; each kernel's map is now
its own store, the pairs together with an index from a key's own
address to its row, built lazily, owned by the pairs it describes,
cleared whenever the pairs are reached for by hand, and grown in step
when a map living alone in its cell is written key by key; a store
holding any key without an address of its own (a thing with its own
`__hash__` and `__eq__`, a subclass of a number) answers "unknown"
for a miss and the old walk decides. A first version of that branch
kept the index in a thread-local map keyed by the pairs' allocation
address with a heuristic freshness check; it was refused, since a
freed map's box is the first thing the allocator hands the next map
and a later map of the same length and last key would have passed
the check while holding other keys. Also waiting: the frozenset kind
(§1r) behind its speed pass, and the function-members work (a
lambda's `__name__`, `__qualname__`, bound-method and instance
writing, `type(f)` as a value, `dir(f)`).

The batch-11 count, taken at 523de8f with the sweep binary copied
aside, stands at stack8 1,268 of 2,536 and microcode7 1,210 of 2,394,
against 1,261 and 1,203 at e1c18cc; no file lost a pass. test_format
rose from one to five on both kernels, and test_complex, test_float
and test_fstring gained one each on both. The fixtures (36 of 36),
the 382-program scratch check and the Python, PHP and Lumen sweeps
were clean on the same tree; the account's weekly limit paused the
work from 18 September, 10:47 UTC, until the 23rd, and the checks
that were still running finished on their own in the meantime. The
three waiting branches are pushed to origin: fix/dict-microcode7
(7d45f4b; fixtures 36 of 36, 170 programs clean, three sweeps clean),
fix/frozenset-kind and fix/function-members (the last two with a
work-in-progress commit each, unbuilt and unverified).

### 1t. Batch 11 merged as #498; batch 12 is the map store

Pull request #498 merged into main at 77b1cb6 with all six jobs green
on 145138b. Batch 12 folds one branch, fix/dict-microcode7. A map was
a row of pairs searched from the front in both full kernels, so a map
built up key by key cost the square of its size: test_dict never got
past its fourteenth test on microcode7 within the 900 s cap (the count
recorded it as running nothing there), and scratch/perf/3, a
hundred-thousand-entry map, timed out on both kernels. Each kernel's
map is now its own store (microcode7 `MapStore`, stack8 `KeyedPairs`):
the pairs together with an index from a key's own address to its
row, built lazily, owned by the pairs it describes, cleared whenever
the pairs are reached for by hand (the store's DerefMut), and grown in
step when a map living alone in its cell is written key by key. A key
without an address of its own (a thing with its own `__hash__` and
`__eq__`, a subclass of a number) is not indexed; the store counts
such keys, and while that count is not zero a miss answers "unknown"
and the old walk decides, so a plain `3` still finds an entry keyed
by an int subclass (scratch/builtin-subclassing/7). test_dict now
finishes on both kernels (76 s stack8, 98 s microcode7 on the debug
binary) with 71 of 142 passing on each, none lost against stack8's
earlier line; perf/3 runs in 5 s and 13 s.

Two designs were refused on the way, and why is worth keeping. The
first kept the index in a thread-local map keyed by the pairs'
allocation address with a heuristic freshness check (same length,
same last key); a freed map's box is the first thing the allocator
hands the next map, so a later map of the same length and last key
would have passed the check while holding other keys, and a lookup
would have answered wrongly and silently. The second, the owned
store, first treated every index miss as proof of absence, which is
false while the map holds keys the index cannot carry; the scratch
check caught it on builtin-subclassing/7 before it was folded.

The batch-12 count, taken at 4caa070, stands at stack8 1,268 of 2,536
and microcode7 1,281 of 2,536: microcode7's total now includes
test_dict's 142 tests, 71 passing, where before it ran nothing; no
file lost a pass. Still open on maps: `.get`, `.pop`, `.setdefault`
and `.update` compare keys with free functions that cannot call
`__eq__`, so `d.get(a)` answers None for the very object `a` used as
a key and `setdefault` can insert a duplicate (fix/thing-keys is
working on it, on top of this store).

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
- **Container rendering is a core label now, and the value's own flag
  survives beneath it.** `system.collection.render` says how a
  collection is written when its text is asked for: `plain` writes each
  member as its own text, which is what every language but Python asks
  and what they all printed before, and `representation` writes each
  member as a representation, which is what Python asks and what CPython
  gives. All six kernels read it, each in its own words, so the examples
  agree again; before, the decision hung on `ext.stmt.class.special`,
  which only the full kernels could see, and the mixed array among the
  examples printed two ways. What a representation is belongs to the
  kernel, as the real rendering does: text between quotes, a collection
  within written the same way again, everything else as it shows.
  Beneath that, the second field of `Value::Collection(cell, quoted)` in
  `stack8` (`value.rs`, read where `display` meets a `Collection`) and
  the matching flag in `microcode7` still mark a container a *Python
  operation* built — `.held(true)` in the eight places, `sorted`,
  `str.split`, `list.copy`, `dict.copy`, the ordering with a key, and
  `list()` of a view, text, cursor or generator, which pass the flag on
  from what they were given. With the label set to `representation` that
  flag no longer decides how a collection prints, and a fixture that
  expects quotes now expects them whatever built the list. Do not mark a
  literal-built row quoted to reach the same end: `sort` in place is a
  core operation all ten languages use, and marking its list would break
  the examples, which is how five earlier attempts died.
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

**The container can also restart with nothing heavy running.** One
restart here came with no release build in progress and the machine
quiet, and it killed six workers mid-task. Nothing was lost only because
every worker had its own branch: their uncommitted work was committed to
those branches and pushed as soon as the session came back. Push every
branch to the remote as soon as it has anything on it, finished or not —
a branch that lives only in this container's `.git` is not saved.

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
