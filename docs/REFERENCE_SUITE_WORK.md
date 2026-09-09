# Working on the reference test suite

`tests/php/` holds tests taken from the reference implementation of PHP, and
`scripts/reference_tests.py` runs every one of them on both full kernels and
writes `tests/REPORT.md`. This page is for anyone taking that work further,
alone or beside others. It says what must be run, where a new word has to be
written down, and which mistakes are quiet rather than loud.

## What must be run, every time

A change is not finished until all of this has been run in this order and each
line has said what it ought to say.

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

The last line is the one that matters most. It lists every test that passed on
both kernels before the change and does not now. It must come back empty. A
test made to pass by skipping it, disabling it, or answering it by name has not
been made to pass.

Note the `git checkout tests/REPORT.md` in the seventh line: it puts the report
back as it stands in the commit so that the diff after is a true before and
after. It also takes the file out of the staging area, so if the report was
staged earlier it must be staged again before committing. A commit carrying a
stale report is easy to make and hard to see.

## The reference implementation is the arbiter, not the test file

`/usr/bin/php` is the reference. Every wording, every edge, every complaint
should be settled by running the case there rather than by reading the test's
expectations and guessing what produced them. `scratchpad/dp.sh` runs a file
through the reference and both kernels and shows where they part:

```
sh dp.sh somefile.php
```

Some tests cannot pass here however good the implementation, because they were
written to run from the reference implementation's own source tree. Two in this
suite ask for files under `sapi/` that this repository does not carry, and one
resolves a relative path against the harness's working directory. Run such a
test's body through `/usr/bin/php` from *this* directory before treating it as a
defect: where the reference fails it too, the test is asking for a layout, not
for behaviour.

## Where a new word has to be written down

Ten languages share these kernels. Behaviour belonging to one language is never
written into a kernel as a test of which language is running; it is put behind a
label in that language's definition, and a kernel that is given no label for it
goes on as before.

A new label has to be written down in five places. Missing one of them fails
quietly — usually as a label that reads as empty, so the behaviour simply never
happens.

1. `langs/extras/<language>.json` — the words themselves.
2. `kernels/stack8/src/lang.rs` — the `EXT_LABELS` roster, and the builtin
   table besides if the label names a builtin, and a field on `Lang` with the
   line that reads it.
3. `kernels/microcode7/src/table.rs` — the roster string, where each label
   carries `:L` for a list of words, `:B` for a switch, `:N` for a count; and
   `BUILTIN_LABELS` besides if it names a builtin, **whose length is written
   into the type and must be raised by hand**.
4. `langs/README.md` — a paragraph of prose saying what the label means. The
   table further down that file is generated; the prose is not.
5. `python3 scripts/lang_table.py`, to regenerate that table.

A message that is plain words becomes a label holding those words. A message
with something dropped into the middle of it stays in the kernel and is put
behind a label, with a two-piece label giving the words before and the words
after; `ext.op.walk.giver.unwalkable` is the pattern to copy.

## Two traps that bite quietly

**The library is built into the binary.** `langs/lib_php/native/*.php` is
gathered by `scripts/port_examples.py` into `langs/lib_php/prelude.rs`, which
`src/main.rs` takes in whole. Editing a library file and rebuilding does
nothing: the generator has to run and `src/main.rs` has to be touched, or the
old library is still in the binary.

```
python3 scripts/port_examples.py && touch src/main.rs && cargo build --release
```

**Two library files may not define the same routine.** The generator refuses
it, because the second quietly displaced the first and broke it from a distance
— this happened once and cost a working date routine. If a routine already
exists, extend it where it is.

## The kernels must not come to resemble one another

`scripts/kernel_independence.py` refuses a run of twelve or more identical
significant lines between any two kernels. The kernels are meant to be
independent accounts of the same language, so the same idea has to be written
twice, in its own shape and its own words each time. Writing one and copying it
across will be refused, and rightly.

## Working beside others

Several people or processes working on this suite at once is worthwhile — the
tests fall into fairly separate lanes — but the ways it goes wrong are quiet
rather than loud. All of the following have happened here.

**Check what you are actually based on.** Run `git log --oneline -3` before
starting and say in your report what it printed. A worktree can be created at a
different commit from the one intended, and everything after follows from that:
baseline numbers that do not match, a "regression" that is nothing of the kind,
a correction sent to someone else that is simply wrong. Regenerate
`tests/REPORT.md` on an untouched tree first and check the `| all |` row is the
one you were told to expect. If it is not, stop.

**A merge of a roster is not a merge of two lists.** The roster lines in
`lang.rs`, `table.rs` and `langs/README.md` are long single lines. Where two
people have both changed one, the merge takes one side entire and the other
side's labels vanish — with no conflict raised, and no failure until something
that used to work stops working. Resolve these by hand every time: take the
line that has the most on it and fold the other side's additions into it, then
check that a label you know was there is still there.

**Two people may add the same label.** The definition files are JSON, and a
merge can leave the same key in twice. JSON with a repeated key parses without
complaint and the last one wins, so nothing fails. After merging a definition
file, check for repeated keys before trusting it.

**Two people may name the same thing differently.** Where one calls a field
`fault_shift` and the other `shift_fault`, both compile in their own tree and
the merge leaves a tree that does not. This one at least is loud.

**Divide by file, not by test.** Two pieces of work that touch the same file
will cost more in merging than they save in running at once. The files most
fought over are `lang.rs`, `table.rs`, the definition JSON and `langs/README.md`,
because nearly every change needs a label; and `engine.rs` and `exec.rs`,
because nearly every change needs the kernels. Say plainly, before starting,
which files a piece of work owns and which it must not touch.

## What is left, and why

At the time of writing 381 of the 397 runnable PHP tests pass on both kernels.
Of what remains:

- Four want text held as bytes rather than as characters. `~"0"` in the
  reference is the single byte `0xCF`; here text is a run of characters, so the
  complement is the character of that number and is written back out as two
  bytes. This reaches `Value::Text` itself, the reading of source, the writing
  of output, and `scripts/reference_tests.py`, which reads a test's expectations
  with unreadable bytes replaced before either kernel sees them.
- Two want the run to count the room it takes. Nothing does.
- Two are asking for the reference implementation's own directory layout.
- One wants the short opening marker, which cannot be settled until a setting
  has been read, and settings are read after the source has been broken into
  words.
- One wants a word for running a command on the host, which is a kernel builtin
  and a question about what the kernels are allowed to do, not about PHP.
