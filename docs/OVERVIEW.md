# Lumen-Lang in one page

Lumen-Lang is an experiment in separating an interpreter's execution substrate
from the languages it runs. One command-line host carries six independent
interpreter kernels, and every language is data: a JSON definition says which
keywords, operators, brackets, literals and builtins spell each construct.
Ten languages run this way (Lumen, a reverse-Polish Lumen, Python, Rust, PHP,
Ruby, Pascal, C, JavaScript and Swift), each as a subset spelled exactly as the
language spells it.

## Six kernels, three shapes

The kernels were built in turn, each asking a different question about how
little an interpreter needs:

| Shape | Kernels | What it is |
|---|---|---|
| Stream | stream35 | a tree of handler nodes, one per construct (35 node types) |
| Microcode (tree) | microcode11, microcode4, microcode7 | a tree reduced to a handful of forms; microcode4 is the floor, where every construct is a call |
| Stack | stack5, stack8 | a flat list of words run by one loop; stack5 is the floor (five words) |

The kernels are separate crates that may not depend on one another, and
`scripts/kernel_independence.py` fails the build if any two of them share a
run of twelve identical source lines: each is an independent account of the
same semantics, not a copy.

## The kernel lab

`docs/KERNEL_LAB.md` records thirteen measured cycles of evolving both shapes
for speed, with predictions written down before each result. What it found:

- **The number of primitives never mattered.** Heap allocations on the hot
  path, value clones and dispatches per source construct did. A primitive
  earns its place only by removing one of those.
- **A few fused instructions beat both extremes.** Starting from five words,
  the evolved stack machine ran a bare loop 7.5 times faster and an
  arithmetic loop 2.2 times faster. An ablation of every added primitive
  settled the counts at eight words (stack8) and seven forms (microcode7).
- **The engineering around the primitives mattered more than the primitives
  on the stack machine, and about half on the tree** — measured by folding
  the non-primitive improvements back into the floor kernels.
- **The price of a tree:** at their floors the two shapes differ by 2 times
  on arithmetic and calls and 6 times on a bare loop. stack8 is the fastest
  kernel on every program and the default.

## How correctness is judged

- **Six kernels must print alike.** Every example program, in every language
  whose definition can spell it, runs on all six kernels and must print what
  stream35 prints: 3,006 checks, all agreeing.
- **The languages' own test suites are the judge.** php-src's `tests/lang`,
  `tests/basic` and `tests/func` pass whole on both full kernels: 399 pass and
  23 are skipped by the tests' own conditions (32-bit only, Windows only,
  missing locales). For Python, the core-language files of CPython 3.14's
  `Lib/test` run unchanged under a full `unittest`: 1,774 of 2,738 tests pass
  on stack8 and 1,781 on microcode7 (65%), up from 1,544 and 1,552 at the
  start of the latest batch. Tests CPython marks as implementation details
  (`@cpython_only`) skip themselves, as they do on PyPy.
- **Nothing is weakened to pass.** The reference tests are never edited, and a
  test made to pass by skipping or special-casing it does not count.

## How the work is done

The project lead directs the work; the implementation is written with AI
assistance, organised like a small engineering team:

- A **coordinator** (Claude Code) picks subjects from the failing tests,
  writes each worker a brief, and verifies every result itself before merging.
- **Workers** (OpenAI Codex with GPT-6 Astra, and Claude models) each take one
  subject in their own git worktree on a 32-core build machine, and deliver
  signed commits on a branch with a report.
- **Every branch is re-verified independently** before it merges: the affected
  reference files on both kernels against the previous count (no passing test
  may regress), a sweep of 1,165 recorded probe programs, the six-kernel
  example gates, and the independence check. Reports have been wrong before;
  only reruns count.
- Verified branches are merged a batch at a time into one pull request, which
  GitHub Actions checks three ways (build and examples, reference suites,
  probe programs) before it merges.
- A telemetry log compares the models on the same kinds of work (time,
  verified gains, problems caught at verification, quota used).

## Where it is heading

The current goal is for every CPython core-language test to pass on both full
kernels or skip by the suite's own rules, as PHP's already do. What remains is
mostly the deeper machinery: garbage collection and weak references, complete
tracebacks, `compile`/`exec`, generators and async, and the long tail of
CPython's exact wording. The history of every batch is in `HISTORY.md`.
