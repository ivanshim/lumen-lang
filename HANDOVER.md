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
