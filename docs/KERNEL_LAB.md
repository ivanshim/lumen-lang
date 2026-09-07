# The Kernel Lab

An experiment in evolving the two kernel shapes for speed. Each lineage
is one mutable specimen under `lab/`, a copy of its floor kernel that is
patched cycle by cycle and measured after each: `lab/stacklab` descends
from stack5, `lab/microlab` from microcode4. Two controls sit beside
them, `lab/stack5a` and `lab/microcode4a`, the floor kernels with the
lab's count-neutral improvements and their counts unchanged, measured in
the section on equal engineering below. The specimens are not held
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

Best of seven, release build, seconds, after every cycle below.

| Program | microcode10 | stack26 | microcode11 | microcode4 | stack5 | microlab (8 forms) | stacklab (14 words) |
|---|---|---|---|---|---|---|---|
| loop | 0.267 | 0.065 | 0.150 | 0.225 | 0.060 | 0.065 | 0.028 |
| loop3m | 0.682 | 0.287 | 0.663 | 1.250 | 0.251 | 0.216 | 0.034 |
| fib | 0.071 | 0.030 | 0.051 | 0.075 | 0.033 | 0.036 | 0.018 |
| sieve | 0.803 | 0.017 | 0.024 | 0.038 | 0.016 | 0.019 | 0.013 |
| strings | 0.046 | 0.037 | 0.040 | 0.045 | 0.036 | 0.040 | 0.034 |
| pi | 0.741 | 0.775 | 0.786 | 0.792 | 0.774 | 0.761 | 0.756 |

Every specimen printed the same as stack5 on all 256 examples in the
embedded languages after every cycle.

## The stack lineage: stack5 to stacklab (14 words)

| Cycle | Change | loop | loop3m | fib | Decision |
|---|---|---|---|---|---|
| 0 | stack5 as is | 0.063 | 0.259 | 0.033 | baseline |
| 1 | two fused words: `UnlessLess a b to` for `Load a; b; Apply lt; Unless`, and `Incr s k` for `Load s; Lit k; Apply add; Store s`; a peephole over each program after assembly, jump targets moved | 0.049 | 0.043 | 0.030 | keep |
| 2 | `Arith op a b`: a binary operation whose operands are bindings, constants or the top of the stack, read in place; the same fast path for machine integers | 0.030 | 0.054 | 0.024 | keep |
| 3 | `Jump to` for `Lit false; Unless`, and an `Arith` followed by a store folds the store into the word | 0.030 | 0.041 | 0.023 | keep |
| 4 | a call moves its arguments from the stack into the frame directly, one allocation instead of two | 0.031 | 0.036 | 0.023 | keep, no measurable gain |
| 5 | `Call slot n` for `Load f; Apply call`; and a function whose value only ever comes from a `return` drops its result slot and the two words that set it up | 0.032 | 0.031 | 0.017 | keep |
| 6 | loops tested at the bottom: `When to` (jump when true) and `WhenLess a b to`, one jump per pass instead of two | 0.032 | 0.031 | 0.017 | keep, small |
| 7 | indexing fused into `Arith`; `PutAt slot` and `PushTo slot` write the array in place with no take and no store; a builtin call's arguments go through one reused buffer instead of a fresh vector | 0.026 | 0.033 | 0.018 | keep |

Cycle 2 first came out at 0.102 on `loop3m`, slower than cycle 1,
because the operands were fetched by cloning; reading them by reference
put it back. A clone of a small integer is a match over every variant
of the value type, and on a loop that does nothing else it is the loop.

The bare loop ends at two words per iteration, `Incr` and `WhenLess`,
and runs 7.5 times faster than stack5. The arithmetic loop is 2.2 times
faster, fib 1.8, sieve 1.2. Cycle 4 moved nothing and cycle 5 moved fib
by a third: the call's cost was the two words of result-slot prologue
and the load of the callee, not the allocation. Strings and pi never
moved and never will from here: one is bound by copying text, the other
by big-integer arithmetic.

## The tree lineage: microcode4 to microlab (8 forms)

| Cycle | Change | loop | loop3m | fib | Decision |
|---|---|---|---|---|---|
| 0 | microcode4 as is | 0.252 | 1.259 | 0.075 | baseline |
| 1 | `Loop` form: a loop runs in the frame it appears in, catching break and continue, instead of a program that calls itself with three frames per iteration | 0.175 | 0.754 | 0.077 | keep |
| 2 | `If` form: the arms are nodes in the same frame, not program values; `and`/`or` read their right side in place | 0.187 | 0.757 | 0.069 | keep, small |
| 3 | `Binary` form: an operation of two operands evaluated without a vector of arguments, with a fast path for machine integers | 0.073 | 0.313 | 0.041 | keep |
| 4 | `Step` form: `x = x + k` steps the binding in place | 0.067 | 0.248 | 0.040 | keep |
| 5 | a call's arguments are evaluated straight into the callee's frame, and a tail call carries a built frame, no vector between | 0.068 | 0.237 | 0.038 | keep, small |
| 6 | a `Binary` operand that is a binding or a constant is read directly, without visiting a node | 0.061 | 0.219 | 0.036 | keep |

Cycle 3 is the finding of the experiment. The four-form tree spent its
time not in the tree walk but in the two heap allocations every `a + b`
made, one for the argument vector and one freed after. Removing them
took the tree from 1.6 times slower than microcode11 to twice as fast,
and by cycle 4 an eight-form tree runs the bare loop as fast as the
five-word and twenty-six-word stack machines. That was predicted not to
happen. Cycles 5 and 6 found the floor of the shape: what is left is the
recursion itself, one `eval` with a large match per node, a borrowed
cell per binding read, and a result wide enough to carry a signal. None
of that is a form, and none of it can go while the tree is walked.

## Predictions against results

- Predicted: the stack lineage ends around eight or nine words, two to
  three times faster than stack5 on loops. Result: fourteen words, 2 to
  7.5 times faster. Right in shape, short in size.
- Predicted: the tree finds its knee at six forms, on par with
  microcode11. Result: the knee was the argument vector, not a form; at
  seven forms the tree is twice as fast as microcode11.
- Predicted: no tree beats the stack machine. Result: a tree of eight
  forms beats the stack floors on the bare loop and ties them on the
  arithmetic loop. The evolved stack machine is still 2 to 6 times ahead
  of it, so the flat list wins, by less than expected.
- Not predicted: cloning a value is a cost worth a whole cycle.

## What the numbers say about primitives

The count never mattered. What mattered, in order: heap allocations on
the hot path (the argument vector, the closures, the frames), value
clones, and the number of dispatches per source construct. A primitive
earns its place by removing one of those, and a primitive that only
renames a shape, like stack26's twenty-one extra words, earns nothing.
How much of each survivor's speed the primitives bought, and how much
the engineering around them would have bought alone, is measured in the
next section: the answer differs by shape.

## The references at equal engineering

stack5 and microcode4 stay the minimalist references. The question after
thirteen cycles was how much of each survivor's speed came from its new
primitives and how much from improvements the lab made alongside them
that never touched the count. To answer it, `lab/stack5a` and
`lab/microcode4a` are the references with every count-neutral
improvement woven in and nothing else: stack5a still has the five words,
microcode4a the four forms.

What stack5a has that stack5 does not: a function whose value only ever
comes from a return drops its result slot; `while` and counted loops are
tested at the bottom, one jump per pass instead of two, the condition
flipped to its complement (`Ge` for `Lt`, a `Not` otherwise) so the
existing `Unless` jumps back up; a builtin's arguments move into one
reused buffer instead of a fresh list; a call moves its arguments from
the stack straight into the frame, one allocation instead of two; and
two machine integers under an operator are computed in place ahead of
the general arithmetic.

What microcode4a has that microcode4 does not: a branch arm or loop body,
a program that owns no names, runs in the frame it closed over and makes
none; an operator's arguments land in a fixed buffer of three, not a
list; a call evaluates its arguments straight into the callee's slots;
the walk up the frames borrows instead of taking a share of each; and
the same integer fast path.

Best of seven, release build, seconds:

| Program | stack5 | stack5a | stacklab | microcode4 | microcode4a | microlab |
|---|---|---|---|---|---|---|
| loop | 0.063 | 0.054 | 0.030 | 0.246 | 0.119 | 0.068 |
| loop3m | 0.262 | 0.213 | 0.035 | 1.344 | 0.538 | 0.242 |
| fib | 0.032 | 0.029 | 0.019 | 0.077 | 0.041 | 0.036 |
| sieve | 0.018 | 0.017 | 0.014 | 0.038 | 0.023 | 0.020 |
| strings | 0.036 | 0.035 | 0.036 | 0.044 | 0.037 | 0.038 |
| pi | 0.821 | 0.787 | 0.755 | 0.799 | 0.788 | 0.769 |

Both variants printed the same as stack5 on all 256 examples.

The two shapes answer differently. On the stack machine the engineering
is worth 1.1 to 1.2 times and the words are worth the rest: 6 times on
the bare loop, 1.8 on the arithmetic loop, 1.5 on calls. Measured before
the integer fast path went in, the allocation and jump work alone was
worth 3 percent. The reason is that the five-word machine's cost is
stack traffic, every operand pushed and popped, and no engineering
around five words removes that; stacklab's operands that read a slot
directly do, and that is a word. On the tree the split is even: the
engineering is worth 1.9 to 2.5 times (the frames for arms and loop
bodies, the argument lists, the frame shares, the fast path), and the
forms are worth 1.15 on calls and arrays and 1.75 to 2.2 on loops. The
earlier attribution of microlab's whole gain to its forms was wrong by
about half; stacklab's stands.

So the corrected reading: for a stack machine, the primitives are the
speed, because only a word can keep an operand off the stack. For a
tree, half the speed was there for the taking under four forms, and the
other half needed a loop and a conditional that are forms of their own.

## Where the lineages stopped, and why

Thirteen cycles. The stack line stopped when the bare loop was two words
and the arithmetic loop was one word per operator: there is no dispatch
left to remove without fusing whole statements, which is a compiler, not
a stack machine. The tree line stopped when every allocation and every
avoidable node visit on the hot path was gone; the rest is the walk. The
two shapes at their floors differ by 2 times on arithmetic and calls and
6 times on a bare loop, and that gap is the price of a tree.

## Next cycles, if there are any

- Stack: a `Value` whose text can grow in place, for the strings
  benchmark; nothing else on the hot paths is left.
- Tree: frames without `RefCell`, and a narrower result type; both are
  executor plumbing rather than forms, and each is worth perhaps a tenth.
- Promotion: a survivor becomes a kernel by being rewritten in its own
  words under `kernels/`, numbered by its count, and held to the
  independence check like the rest. Until then the specimens stay in
  `lab/`.
