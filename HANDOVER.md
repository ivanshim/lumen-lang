# Handover: PHP finished, Python next

## Where things stand

Branch `claude/codebase-familiarization-t6vjhi`, head `85e64c8`, pushed.

PHP is complete against the reference suite:

```
| all | 422 | pass 399, differs 0, error 0, skipped 23 | pass 399, differs 0, error 0, skipped 23 |
```

399 of 399 runnable tests pass on both full kernels, with zero differences and
zero errors. The 23 passed over are passed over honestly — ten want a 32-bit
machine, eight want Windows, two want a `php-cgi` binary, three want a locale
this machine does not carry — and the reference declines every one of them here
in the same words. None is to be won back by writing anything.

The suite went from 363 to 399 in one session. Every change was verified with
the full sequence below, and no test that passed at any point stopped passing.

## Read these first

- `docs/REFERENCE_SUITE_WORK.md` — what must be run, the five places a label has
  to be written down, and the mistakes that are quiet rather than loud. It was
  written from mistakes actually made here, not from imagination. **Anyone
  touching this repository should read it before touching it.**
- `langs/README.md` — the definition format and every label, with the ten
  languages side by side.
- `README.md` — the architecture: one host, six independent kernels.

## What must be run, every time

A change is not finished until all of this has run and each line says what it
ought to.

```
RUSTFLAGS="-D warnings" cargo build
python3 scripts/kernel_independence.py           # must say 0 problem(s)
python3 scripts/port_examples.py
python3 scripts/lang_table.py
touch src/main.rs && cargo build --release
TEST_QUIET=1 ./test.sh --lang all                # must be 3006 of 3006
git checkout tests/REPORT.md && python3 scripts/reference_tests.py
grep -c "Kernel disagreements" tests/REPORT.md   # must be 0
git diff tests/REPORT.md | grep -E "^-.*\| pass \| pass \|"   # must print nothing
```

The last line is the regression check and it is absolute.

**Never write `>/dev/null 2>&1` on the reference run.** Where it stops before it
begins it writes no report, the comparison after it is empty, and an empty
comparison is exactly what success looks like. Two changes were called verified
here when nothing whatever had been run.

## The next piece of work: Python on stack8 and microcode7

### What already exists

- `langs/python.json` — a full definition, and unlike PHP's it is **compiled
  into all six kernels** (`include_str!` in each kernel's `lib.rs`).
- `langs/lib_python/` — the library as Python spells it; 70 of the 136 shared
  functions are carried.
- 58 ported example programs under `examples/python/`, all passing on all six
  kernels as part of the 3006 example runs.

### What does not

`tests/python/` holds 50 core-language files taken from CPython's `Lib/test`.
**All 50 fail on both kernels**, and every one of them fails in the reader,
before a line is ever run:

| count | what the reader stops at |
|---|---|
| 16 | `@` — decorators |
| 10 | `:` — slices and annotations |
| 9 | `\` — a line carried on to the next |
| 4 | `~` |
| 3 | `&` |
| 2 | `,` in a place the reader does not expect one |
| 1 | `*` before a parameter name |
| 1 | `?` |
| 1 | indentation the reader will not have |
| 1 | an expression the reader wants and does not find |

### Read this before promising anything

**This is not the same shape of job as PHP was, and it must not be scoped as
though it were.** The PHP tests are `.phpt` files carrying an `--EXPECT--`
section: a test passes when its output matches. The Python files are `unittest`
modules with no expected output at all. `scripts/reference_tests.py` presently
runs one and, if it exits cleanly, records `ran to the end without asserting
anything`.

So "make the 50 pass" quietly means: the whole reader, most of the language,
`unittest`, and a large part of the standard library. That is a long road, and
anyone who plans it as "PHP took a day, so Python will" is going to be wrong.

**Stage it, and say plainly which stage is being worked on:**

1. **The reader.** Get all 50 files read without a reader complaint — decorators,
   slices, continuations, the missing operators, starred parameters. This alone
   is a real, measurable milestone: the failure table above should empty.
2. **Running.** Get them to run far enough to fail on something that is not the
   reader. Expect a long tail of missing builtins and syntax.
3. **Asserting.** Decide what "pass" even means for a `unittest` module here,
   and whether the harness should learn to run one. That decision belongs to the
   repository's owner, not to an agent.

Stage 1 is the right target for the first several sessions. Do not let a
coordinator report progress against stage 3 while doing stage 1.

## How this session is to be run

**Claude Fable 5.1 coordinates. It writes no code.** Every edit to every file is
made by Codex. The coordinator's job is to decide what to do, split it so the
pieces do not collide, brief each piece precisely, verify what comes back
against the sequence above, and commit and push.

Start it on the Mac with:

```
claude --remote-control
```

optionally with a name (`claude --remote-control lumen`), from a checkout of
this branch.

### The Codex call — FILL THIS IN FIRST

This is deliberately left blank rather than guessed at. The container this
handover was written in has no `codex` binary, so no invocation could be
verified, and a command line that fails on first use is worse than none.

**Before any work begins, the coordinator must:**

1. Run `codex --help` (and `codex exec --help` if that exists) and read it.
2. Determine the switch that selects the model, and set it to the agreed one.
3. Determine how to run it non-interactively against a working directory, and
   how its output comes back.
4. **Record the exact working command in this file, in the block below**, so
   nobody has to rediscover it.

```
# The Codex invocation actually used (fill in and commit):
#
#   codex ... --model <AGREED MODEL> ...
#
# Model agreed with the repository's owner: ______________________
#
# NOTE: the handover brief named "gpt 5.1 astra" in one place and
# "gpt 6 astra" in another. Settle which before starting; do not guess.
```

### Rules for the coordinator

1. **Write no code.** Not a one-line fix, not a typo, not a `sed`. If it changes
   a tracked file that is not a commit message, it goes to Codex. Running builds,
   tests, `git`, and reading files is the coordinator's own work.
2. **Divide by file, never by test.** Two pieces of work touching one file cost
   more in merging than they save in running at once. The files most fought over
   are `kernels/stack8/src/lang.rs`, `kernels/microcode7/src/table.rs`, the
   definition JSON, and `langs/README.md`, because nearly every change needs a
   label; then `engine.rs` and `exec.rs`. State in every brief which files that
   piece owns and which it must not touch.
3. **Give Codex the constraints, every time.** It cannot infer them:
   - Ten languages share these kernels. Behaviour belonging to one goes behind a
     label in that language's definition, never a test of which language is
     running.
   - A new label must be written down in **five** places (see
     `docs/REFERENCE_SUITE_WORK.md`). Missing one fails **silently**, as a label
     that reads as empty.
   - `scripts/kernel_independence.py` refuses twelve or more identical
     significant lines between any two kernels. The same idea must be written
     twice, in its own shape and words each time. This catches a great deal of
     copy-and-paste and is not negotiable.
   - Prose style is plain, slightly archaic English. Match the surrounding
     comments. **Never name any AI model, agent, or tool in code, comments, or
     commit messages.**
4. **Verify everything yourself.** Do not take a report's word for a number. Run
   the sequence. Read the last lines the reference run prints.
5. **Check what a worker was actually based on.** In the session that finished
   PHP, *every single* delegated piece of work started from the wrong commit
   despite being told the right one, and several then reported baseline numbers
   that were simply wrong — including a "correction" sent back to the
   coordinator that was itself mistaken. Have each piece print its base commit
   and reproduce the current `| all |` row on an untouched tree **before** it
   changes anything.
6. **Merging a roster is not merging two lists.** The label rosters are long
   single lines. Where two pieces of work both changed one, the merge takes one
   side entire and the other side's labels vanish, with no conflict raised and
   no failure until something that used to work stops. Resolve those by hand and
   check afterwards that a label you know was there still is.
7. **Definition files are JSON, and a merge can leave a key in twice.** It parses
   fine, the last one wins, nothing fails. Check for repeated keys after merging.
8. **Report honestly.** If a thing cannot be reached, say so and say what it
   would take. An honest "not reachable without X" is worth more than a hack, and
   far more than a number improved by reclassifying a failure.

### One trap that is specific to Python here

`langs/python.json` is **compiled into every kernel**, not read from disk like
PHP's. A change to it needs a rebuild to take effect, and it is read by all six
kernels — including the four reference kernels, which read past `ext.*` labels
entirely. Anything put behind an `ext.*` label will therefore be honoured by
stack8 and microcode7 and ignored by the other four. That is the intended
meaning of an extension label, but it means the six can disagree on output if a
change is not thought through, and `./test.sh --lang all` compares all six
against each other. That exact mistake broke 12 of the 3006 example runs during
the PHP work.

---

## The prompt to start the coordinator with

Paste everything below into the Fable 5.1 remote-control session.

---

You are coordinating work on the `lumen-lang` repository, on branch
`claude/codebase-familiarization-t6vjhi` (head `85e64c8`). Read `HANDOVER.md`
first, then `docs/REFERENCE_SUITE_WORK.md`, then `langs/README.md`.

**Your role is orchestration only. You write no code.** Every change to every
tracked file is made by Codex, called out to as described in `HANDOVER.md`. You
decide what to do, split it so pieces do not collide, brief each piece
precisely, verify what comes back, and commit and push. Reading files, running
builds and tests, and using git are yours; editing a tracked file is not. If you
catch yourself about to make a one-line fix, that is a Codex call.

**Before anything else:** settle the Codex invocation. Run `codex --help`,
determine the switch that selects the model and how to run it non-interactively,
confirm the model with the repository's owner if it is not already recorded, and
write the working command into the marked block in `HANDOVER.md`. Do not begin
implementation work until a Codex call has succeeded end to end on something
trivial and you have seen its edit land.

**The work: implement Python on the two full kernels, stack8 and microcode7.**

The state is described in `HANDOVER.md` under "The next piece of work". In
short: `langs/python.json` and `langs/lib_python/` already exist and 58 example
programs already pass on all six kernels, but all 50 files in `tests/python/`
fail on both full kernels, every one of them in the reader, before a line runs.

**Work stage 1 only for now: the reader.** The goal is that all 50 files are
read without a reader complaint. The failure table in `HANDOVER.md` lists what
stops it — decorators, slices and annotations, line continuations, `~`, `&`,
starred parameters, and a few one-offs. Getting that table to empty is the
milestone. Do not report progress as though the tests were passing; they will
not be, and `HANDOVER.md` explains why that is a much longer road.

Suggested first moves:

1. Reproduce the baseline yourself. Run the full verification sequence on an
   untouched tree and confirm `pass 399, differs 0, error 0, skipped 23` for PHP
   and 50 errors for Python. Do not proceed until you have seen it.
2. Group the 50 failures by cause, not by file. Several causes account for most
   of them, and each is one piece of work.
3. Take the largest cause first, on its own, end to end — brief Codex, verify,
   commit, push — before running anything in parallel. Learn what a good brief
   for Codex looks like on a small piece before betting a big one on it.
4. Only then fan out, and only along boundaries that do not share a file.

Rules that are not yours to relax: the verification sequence in `HANDOVER.md`
runs in full before every commit; the regression check must come back empty; the
two kernels must be written independently and
`scripts/kernel_independence.py` must report `0 problem(s)`; no label may be
added without all five registrations; no AI model, agent or tool may be named in
any committed artifact; and the reference run's stderr is never thrown away.

Report progress as: what was attempted, what the verification actually printed,
what landed, and what did not and why. If something is not reachable, say so
plainly and say what it would take.
