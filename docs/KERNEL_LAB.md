# The Kernel Lab

An experiment in evolving the two kernel shapes for speed. Each lineage
is one mutable specimen under `lab/`, a copy of its floor kernel that is
patched cycle by cycle and measured after each: `lab/stacklab` descends
from stack5, `lab/microlab` from microcode4. The specimens are not held
to `scripts/kernel_independence.py` against their ancestors; they are
specimens, not kernels. The measurements are the product.

`scripts/bench.sh [N] [kernel...]` times every program under `bench/` on
every kernel, best of N release runs. The six programs: `loop` (300k
iterations of arithmetic), `loop3m` (a bare loop, three million times),
`fib` (recursive, the cost of a call), `sieve` (array reads and writes),
`strings` (a string built up piece by piece), `pi` (big integers; no
kernel design can touch it, so it is the control).

The rule of the experiment was purity: the tree stays a tree the executor
walks, and the stack machine stays a list of words over one stack. Each
cycle adds a form or a word, or changes how one is run, and the number in
the specimen's name would be its primitive count if it were promoted.

## Results

Best of five, release build, seconds.

| Program | stack5 | stacklab | microcode4 | microcode11 | microlab |
|---|---|---|---|---|---|
| loop | 0.061 | 0.031 | 0.235 | 0.159 | 0.067 |
| loop3m | 0.267 | 0.040 | 1.321 | 0.677 | 0.248 |
| fib | 0.032 | 0.023 | 0.074 | 0.055 | 0.040 |
| sieve | 0.018 | 0.015 | 0.040 | 0.025 | 0.018 |
| strings | 0.037 | 0.036 | 0.047 | 0.040 | 0.039 |
| pi | 0.785 | 0.795 | 0.803 | 0.817 | 0.833 |

Every specimen printed the same as stack5 on all 256 examples in the
embedded languages after every cycle.

## The stack lineage: stack5 to stacklab (9 words)

| Cycle | Change | loop | loop3m | fib | Decision |
|---|---|---|---|---|---|
| 0 | stack5 as is | 0.063 | 0.259 | 0.033 | baseline |
| 1 | two fused words: `UnlessLess a b to` for `Load a; b; Apply lt; Unless`, and `Incr s k` for `Load s; Lit k; Apply add; Store s`; a peephole over each program after assembly, jump targets moved | 0.049 | 0.043 | 0.030 | keep |
| 2 | `Arith op a b`: a binary operation whose operands are bindings, constants or the top of the stack, read in place; the same fast path for machine integers | 0.030 | 0.054 | 0.024 | keep |
| 3 | `Jump to` for `Lit false; Unless`, and an `Arith` followed by a store folds the store into the word | 0.030 | 0.041 | 0.023 | keep |
| 4 | a call moves its arguments from the stack into the frame directly, one allocation instead of two | 0.031 | 0.036 | 0.023 | keep, no measurable gain |

Cycle 2 first came out at 0.102 on `loop3m`, slower than cycle 1,
because the operands were fetched by cloning; reading them by reference
put it back. A clone of a small integer is a match over every variant
of the value type, and on a loop that does nothing else it is the loop.

The bare loop ends at three words per iteration: `UnlessLess`, `Incr`,
`Jump`, and runs 6.5 times faster than stack5. The arithmetic loop halved.
The call path did not move: fib's cost is elsewhere (the frame, the
result slot, the trampoline on return), and would be the next cycle.

## The tree lineage: microcode4 to microlab (8 forms)

| Cycle | Change | loop | loop3m | fib | Decision |
|---|---|---|---|---|---|
| 0 | microcode4 as is | 0.252 | 1.259 | 0.075 | baseline |
| 1 | `Loop` form: a loop runs in the frame it appears in, catching break and continue, instead of a program that calls itself with three frames per iteration | 0.175 | 0.754 | 0.077 | keep |
| 2 | `If` form: the arms are nodes in the same frame, not program values; `and`/`or` read their right side in place | 0.187 | 0.757 | 0.069 | keep, small |
| 3 | `Binary` form: an operation of two operands evaluated without a vector of arguments, with a fast path for machine integers | 0.073 | 0.313 | 0.041 | keep |
| 4 | `Step` form: `x = x + k` steps the binding in place | 0.067 | 0.248 | 0.040 | keep |

Cycle 3 is the finding of the experiment. The four-form tree spent its
time not in the tree walk but in the two heap allocations every `a + b`
made, one for the argument vector and one freed after. Removing them
took the tree from 1.6 times slower than microcode11 to twice as fast,
and by cycle 4 an eight-form tree runs the bare loop as fast as the
five-word and twenty-six-word stack machines. That was predicted not to
happen.

## Predictions against results

- Predicted: the stack lineage ends around eight or nine words, two to
  three times faster than stack5 on loops. Result: nine words, 2 to 6.5
  times faster. Right in shape, short in size.
- Predicted: the tree finds its knee at six forms, on par with
  microcode11. Result: the knee was the argument vector, not a form; at
  seven forms the tree is twice as fast as microcode11.
- Predicted: no tree beats the stack machine. Result: a tree of eight
  forms ties the stack floors on the bare loop and is within 10 percent
  on the arithmetic loop. The evolved stack machine is still 2 to 6
  times ahead of it, so the flat list wins, by less than expected.
- Not predicted: cloning a value is a cost worth a whole cycle.

## What the numbers say about primitives

The count never mattered. What mattered, in order: heap allocations on
the hot path (the argument vector, the closures, the frames), value
clones, and the number of dispatches per source construct. A primitive
earns its place by removing one of those, and a primitive that only
renames a shape, like stack26's twenty-one extra words, earns nothing.

## Next cycles, if there are any

- Stack: the call path (frame reuse, no result slot for a function that
  always returns), and a bottom-tested loop that fuses the compare and
  the jump back.
- Tree: frames without `RefCell`, and a `Call` form for calls by name
  whose arguments go straight into the new frame.
- Promotion: a survivor becomes a kernel by being rewritten in its own
  words under `kernels/`, numbered by its count, and held to the
  independence check like the rest. Until then the specimens stay in
  `lab/`.
