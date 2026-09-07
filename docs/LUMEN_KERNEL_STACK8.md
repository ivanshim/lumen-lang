# Stack Kernel, Third Design (stack8)

`kernels/stack8/` is stack5 with three words added, the three the kernel
lab (`docs/KERNEL_LAB.md`) measured worth a word. The five of stack5 are
still the whole language of the compiler: every construct is first
assembled from them, and a peephole over each finished routine then
replaces runs of them with one fused word.

```
Const v            push a constant
Read cell          push a binding's value
Write cell         pop into a binding
Act op, n          pop n arguments, apply a kernel action, push its result
Skip to            pop; when the value is not true, continue at `to`

Dyad op, a, b      an operator on two operands, each a cell, a constant or
                   the stack's top; the result pushed
Bump cell, k       a cell stepped by a constant, in place
SkipCmp op, a, b, to   a comparison of two operands; when it fails,
                   continue at `to`
```

Nothing in `kernels/stack8/src/` names a keyword, an operator, a comment
marker or a function; every spelling comes from a definition in `langs/`.

## The peephole

| Run of the five | Fused to |
|---|---|
| `Read a; b; Act cmp; Skip to` (b a cell or a constant, cmp a comparison) | `SkipCmp cmp, a, b, to` |
| `Read x; Const k; Act add; Write x` (k a machine integer) | `Bump x, k` |
| `Read a; b; Act op` (a and b cells or constants, op arithmetic or a comparison) | `Dyad op, a, b` |
| `b; Act op` (b a cell or a constant, the other operand on the stack) | `Dyad op, top, b` |

A run is left alone if a jump lands inside it, and jump targets move with
the words. A `Dyad` reads its cell operands by reference and never
pushes or pops them: that is the whole reason it exists. Two machine
integers are computed in place; anything else, or an overflow, takes the
general arithmetic.

Loops are tested at the bottom, as in stack5, with the condition flipped
to its complement so the existing `Skip` jumps back up; a counted loop's
bottom therefore fuses to one `SkipCmp ge, v, #end, top`, and the bare
loop of the benchmarks runs as `Bump` and `SkipCmp`, two words per pass.

## The stages

| Stage | File | What it does |
|---|---|---|
| Lex | `lex.rs` | text to tokens |
| Layout | `layout.rs` | indentation to block tokens; line ends inside brackets dropped |
| Compile | `compile.rs` | tokens to the five words in one pass; names to cells; then the peephole |
| Run | `engine.rs` | one loop, eight match arms, one stack; fused words read cells by reference |

Supporting: `code.rs` (the eight words, the actions, the routine),
`lang.rs` (the definition as data), `value.rs`, `arith.rs`.

## What was left out, and why

The lab's specimen had fourteen words. The ablation in the notebook
switched each off on its own: `Jump` and a store folded into `Dyad`
measured nothing; `When` (a jump on true) measured 2.3 times, but only
because the specimen had no other way to jump back on a true test, and
flipping the comparison costs nothing; `UnlessLess` and `WhenLess`
collapse into `SkipCmp` on any comparison; `Call`, `PutAt` and `PushTo`
each measured 14 percent on the one program that uses them, under the
fifth a word has to earn. So eight.

## Cost

Best of five, release build, seconds:

| Program | stack5 | stack8 | microcode4 | microcode7 | microcode11 |
|---|---|---|---|---|---|
| loop | 0.055 | 0.034 | 0.125 | 0.067 | 0.190 |
| loop3m | 0.228 | 0.045 | 0.590 | 0.237 | 0.801 |
| fib | 0.032 | 0.025 | 0.047 | 0.045 | 0.068 |
| sieve | 0.021 | 0.016 | 0.029 | 0.024 | 0.030 |
| strings | 0.041 | 0.039 | 0.047 | 0.043 | 0.045 |
| pi | 0.861 | 0.876 | 0.858 | 0.873 | 0.853 |

Five times stack5 on the bare loop, 1.6 on the arithmetic loop, 1.3 on
calls and arrays, level on strings and big integers. The fastest kernel
on every program, and the host's default.

## Relationship to the other kernels

The kernels never import each other; `scripts/kernel_independence.py`
checks every pair, and this kernel is written in its own words, not
stack5's. The test suite compares it with the others on every
program under `examples/`; all 498 print the same. microcode7 is the same
promotion on the tree.

Being a full kernel, it also reads the `ext.` labels a definition may add
beyond the 133 core labels (see `langs/README.md`): an epilogue marker,
`echo`, bracketless builtin calls, `++`/`--`, interpolating strings
(which the scanner turns into a bracketed concatenation before the
compiler sees them), the three-part `for` (its test at the bottom like a
while loop, `continue` landing on the step), `switch` (each case a test
that skips to the next test, each section falling into the next over
that test), the ternary as a conditional jump inside an expression,
`static` (a hidden global, its setting assembled in the unit around the
function), `global`, `define`/`const` (a store to the global at build
time), `var_dump`, compound assignment, `break n`, hoisted top-level
functions (their instrs lifted to the front of the unit) and exponent
literals. The reference kernels skip those labels, so the reference
suites under `tests/` run on this kernel and microcode7 only.

Classes are values here too: a class declaration forges one from a plan
the compiler builds and binds it to its name, so `new C` and `C::X` are
reads. Methods are ordinary programs whose first parameter is the object.
A raised value travels as a fault the run loop catches: `Guard` marks
where a catch stands and how deep the stack was, `Unguard` takes the mark
away, and an `Act` that raises unwinds to the nearest guard. A last part
is written twice, once for each way out, and a return inside a try writes
its value aside, runs the last parts, and only then leaves.

It also holds a map, which the reference kernels do not: `Value::Map`, an
ordered list of keys with their values, beside the plain array. A literal
gathers ties into a map and everything else into an array, so a list
keeps its representation and its speed; writing a key an array does not
hold turns that array into a map. `foreach` and `for v in a` walk either
by position, with `Extent`, `KeyAt` and `ValueAt` doing the reading.
