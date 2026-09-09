# Handover — read this and begin

You are coordinating work on `lumen-lang`. This document is your whole
briefing: read it and start. Nothing else is coming.

**Branch:** `claude/codebase-familiarization-t6vjhi` **Head:** `fd90d29`

---

## 1. Your role: you orchestrate, you do not code

**Every change to every tracked file is made by Codex.** You decide what to do,
split it so the pieces do not collide, brief each piece precisely, verify what
comes back, and commit and push.

Yours: reading files, running builds and tests, git, judgement, briefs.
Not yours: editing a tracked file. Not a one-line fix. Not a typo. Not a `sed`.
Writing a commit message is yours; changing a line of code is not.

The moment you catch yourself about to make a small edit "since it is quicker"
— that is a Codex call. This rule has no exceptions, and it is the one that
decays first if you let it.

### Calling Codex

The model is **GPT-6 Astra**. Codex is already installed and connected on this
machine, so you are not setting it up from nothing.

Before any real work, prove the path end to end:

1. Run `codex --help` (and `codex exec --help` if present) and read it.
2. Confirm the switch that selects the model, and set it to **GPT-6 Astra**.
3. Confirm how to run it non-interactively against this working directory, and
   how its output and its edits come back to you.
4. Send it something trivial — a comment reworded in a file you then revert —
   and watch the edit actually land.
5. **Write the exact working invocation into the block below and commit it**, so
   it never has to be rediscovered.

```
# The Codex invocation actually used — fill in and commit:
#
#   codex ... --model <gpt-6-astra as the CLI spells it> ...
#
# Verified working on: ____________
```

Do not begin implementation until step 4 has succeeded. A coordinator that
cannot reliably deliver an edit has nothing to coordinate.

---

## 2. Where things stand

PHP is finished against its reference suite:

```
| all | 422 | pass 399, differs 0, error 0, skipped 23 |
```

399 of 399 runnable tests pass on both full kernels — zero differences, zero
errors. The 23 passed over are honest: ten want a 32-bit machine, eight want
Windows, two want a `php-cgi` binary, three want a locale this machine lacks.
The reference declines every one of them here in the same words. **None can be
won back by writing code**; do not spend time on them.

That suite went from 363 to 399 in one session, with no test ever regressing.

---

## 3. Read these before touching anything

- `docs/REFERENCE_SUITE_WORK.md` — what must be run, the five places a label has
  to be written down, and the mistakes that are quiet rather than loud. Written
  from mistakes actually made, not imagined. **Non-optional.**
- `langs/README.md` — the definition format and every label, ten languages side
  by side.
- `README.md` — the architecture: one host, six independent kernels.

---

## 4. The verification sequence

Nothing is finished until all of this has run and each line says what it should:

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

The last line is the regression check. It is absolute: no test that passed on
both kernels may stop passing, for any reason, ever.

**Never write `>/dev/null 2>&1` on the reference run.** If it stops before it
begins — a moved path, a definition that will not parse — it writes no report,
the comparison after it is empty, and *an empty comparison is exactly what
success looks like*. Two changes were called verified here when nothing at all
had run. Let it speak and read its last lines.

---

## 5. The work: Python on stack8 and microcode7

### What already exists

- `langs/python.json` — a full definition, **compiled into all six kernels**
  (`include_str!` in each kernel's `lib.rs`), unlike PHP's which is read from
  disk.
- `langs/lib_python/` — the library as Python spells it; 70 of the 136 shared
  functions carried.
- 58 example programs under `examples/python/`, passing on all six kernels as
  part of the 3006 example runs.

### What does not

`tests/python/` holds 50 core-language files taken from CPython's `Lib/test`.
**All 50 fail on both kernels, every one in the reader, before a line runs:**

| count | what the reader stops at |
|---|---|
| 16 | `@` — decorators |
| 10 | `:` — slices and annotations |
| 9 | `\` — a line carried to the next |
| 4 | `~` |
| 3 | `&` |
| 2 | `,` where the reader expects none |
| 1 | `*` before a parameter name |
| 1 | `?` |
| 1 | indentation the reader will not have |
| 1 | an expression wanted and not found |

### Scope this honestly — it is not the job PHP was

PHP's tests are `.phpt` files carrying an `--EXPECT--` section: match the output
and you are done. **The Python files are `unittest` modules with no expected
output at all.** `scripts/reference_tests.py` currently runs one and, if it
exits cleanly, records `ran to the end without asserting anything`.

So "make the 50 pass" quietly means: the whole reader, most of the language,
`unittest`, and a large part of the standard library. Anyone planning this as
"PHP took a session, so Python will" is going to be wrong by a wide margin.

**Three stages. Say which one you are working on, always:**

1. **The reader.** All 50 files read without a reader complaint — decorators,
   slices, continuations, the missing operators, starred parameters. The table
   above should empty. This is measurable and it is real progress.
2. **Running.** They run far enough to fail on something that is not the reader.
   Expect a long tail of missing builtins and syntax.
3. **Asserting.** Decide what "pass" even means for a `unittest` module here, and
   whether the harness should learn to run one. **That decision belongs to the
   repository's owner, not to you.** Bring it to them; do not settle it yourself.

**Work stage 1 only until it is done.** Do not report progress as though tests
were passing. They will not be, and that is expected.

---

## 6. Rules for delegating

1. **Divide by file, never by test.** Two pieces touching one file cost more in
   merging than they save in running at once. Most fought over:
   `kernels/stack8/src/lang.rs`, `kernels/microcode7/src/table.rs`, the
   definition JSON, `langs/README.md` — because nearly every change needs a
   label — then `engine.rs` and `exec.rs`. **State in every brief which files
   that piece owns and which it must not touch.**

2. **Give Codex the constraints every time. It cannot infer them:**
   - Ten languages share these kernels. Behaviour belonging to one goes behind a
     label in that language's definition, **never** a test of which language is
     running.
   - A new label must be written in **five** places (see
     `docs/REFERENCE_SUITE_WORK.md`). Missing one fails **silently**, as a label
     that reads as empty.
   - `scripts/kernel_independence.py` refuses twelve or more identical
     significant lines between any two kernels. The same idea must be written
     twice, in its own shape and words each time. Not negotiable.
   - Prose style: plain, slightly archaic English, matching the surrounding
     comments.
   - **Never name any AI model, agent, or tool in code, comments, or commit
     messages.** Chat only.

3. **Make each piece prove its base.** In the session that finished PHP, *every
   single* delegated piece began from the wrong commit despite being told the
   right one, and several then reported baseline numbers that were simply wrong
   — including a "correction" sent back to the coordinator that was itself
   mistaken. Require each piece to print its base commit and reproduce the
   current `| all |` row on an untouched tree **before** changing anything.

4. **Verify everything yourself.** Never take a report's word for a number. Run
   the sequence. Read what the reference run actually printed.

5. **Merging a roster is not merging two lists.** The label rosters are long
   single lines. Where two pieces both changed one, the merge takes one side
   entire and the other's labels vanish — no conflict raised, no failure until
   something that worked stops. Resolve by hand, then check a label you know was
   there still is.

6. **Definition files are JSON, and a merge can leave a key in twice.** It parses,
   the last wins, nothing fails. Check for repeated keys after every merge.

7. **Report honestly.** If something cannot be reached, say so and say what it
   would take. An honest "not reachable without X" is worth more than a hack —
   and far more than a number improved by reclassifying a failure as a skip.

---

## 7. A trap specific to Python here

`langs/python.json` is **compiled into every kernel**, not read from disk like
PHP's. Two consequences:

- A change to it needs a rebuild to take effect.
- It is read by all six kernels — and the four reference kernels (stream35,
  microcode11, microcode4, stack5) **read past `ext.*` labels entirely**.

So anything put behind an `ext.*` label is honoured by stack8 and microcode7 and
ignored by the other four. That is what an extension label means, but it means
the six can disagree on output if a change is not thought through — and
`./test.sh --lang all` compares all six against each other. That exact mistake
broke 12 of the 3006 example runs during the PHP work.

---

## 8. Your first moves

1. Reproduce the baseline yourself. Run the full verification sequence on an
   untouched tree. Confirm PHP reads `pass 399, differs 0, error 0, skipped 23`
   and Python shows 50 errors on both kernels. **Do not proceed until you have
   seen it with your own eyes.**
2. Prove the Codex path end to end (§1) and record the invocation.
3. Group the 50 Python failures **by cause, not by file**. A few causes account
   for most of them; each cause is one piece of work.
4. Take the largest cause — decorators, 16 files — **on its own, end to end**:
   brief, verify, commit, push. Learn what a good Codex brief looks like on one
   piece before betting several on it.
5. Only then fan out, and only along boundaries that share no file.

---

## 9. How to report

Each time: what was attempted, **what the verification actually printed**, what
landed, what did not and why. Name the stage you are working in. If something is
out of reach, say so plainly and say what it would take.

Do not report a number you have not seen yourself.
