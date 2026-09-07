# Microcode Kernel, Fourth Design (microcode7)

`kernels/microcode7/` is microcode4 with three forms added, the three
the kernel lab (`docs/KERNEL_LAB.md`) measured worth a form. The tree
has seven forms:

```
Const     a constant; a routine among them
Read      a binding
Write     a binding written
Apply     a primitive of the kernel, or a routine value, applied

Cycle     a loop: test, body, an optional step, tested before or after
Dyad      an operator on two operands, each a form, a binding or a constant
Bump      a binding stepped by a constant, in place
```

Everything else is still a call, as in microcode4: `a; b` is `seq(a,
b)`, `if` is `choose(c, «A», «B»)` with routine values for arms, a bare
block is a routine run at once, `and` and `or` take the right side as a
routine, `yield`, `leave` and `resume` are calls that unwind. What
changed is that a loop is no longer a routine that calls itself, an
operator no longer gathers its arguments into a list, and `x = x + k`
no longer visits three forms.

Nothing in `kernels/microcode7/src/` names a keyword, an operator, a
comment marker or a function; every spelling comes from a definition.

## The stages

| Stage | File | What it does |
|---|---|---|
| Scan | `scan.rs` | text to tokens |
| Indent | `indent.rs` | indentation to block tokens; line ends inside brackets dropped |
| Build | `build.rs` | tokens to the seven forms; names resolved to environments and cells; RPLumen read with a symbolic stack |
| Exec | `exec.rs` | the seven forms evaluated, with tail calls |

Supporting: `form.rs` (the forms), `table.rs` (the definition as a
table of tags), `data.rs` (values and environments), `math.rs`.

## What holds it up

- **Routine values are closures**, as in microcode4, and a routine that
  holds no names (a branch arm, an `and` or `or` right side) makes no
  environment: it runs in the one it closed over. The builder counts
  depth over environment-making layers only. That is why `choose` with
  routine arms costs about what a branch form would, and why there is
  no `If` form.
- **Cycle** runs its test, body and step in the frame it appears in,
  catching `leave` and `resume` itself. The body is read in place, not
  as a routine.
- **Dyad** reads an operand that is a binding or a constant directly,
  with no visit to a form, and computes two machine integers in place.
- **Tail calls and escapes** are microcode4's: a routine in tail position
  replaces the running one in the same native frame; `yield`, `leave`
  and `resume` travel up as the error side of a result until a routine
  that traps them stops them.

## What was left out, and why

The lab's specimen had eight forms. The ablation switched each off on
its own: `If` measured 7 percent on calls and nothing on loops, once arms
made no frame; `Bump` measured 1.3 times on the bare loop; `Cycle` 1.8;
`Dyad` 3.7. So seven.

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

Two and a half times microcode4 on the bare loop, twice on the
arithmetic loop, ahead of microcode11 everywhere, and behind stack8 by
the price of a tree: 5 times on the bare loop, twice on the arithmetic
loop, 1.8 on calls, 1.5 on arrays.

## Relationship to the other kernels

The kernels never import each other; `scripts/kernel_independence.py`
checks every pair, and this kernel is written in its own words, not
microcode4's. All 498 example programs print the same on this kernel as
on the other five. stack8 is the same promotion on the stack machine.

Being a full kernel, it also reads the `ext.` labels a definition may add
beyond the 133 core labels (see `langs/README.md`): an epilogue marker,
`echo`, bracketless builtin calls, `++`/`--` (a statement folds into a
`Bump`; `x++` in an expression keeps the old value aside in a hidden
binding), interpolating strings (which the scanner turns into a
bracketed concatenation), the three-part `for` (a `Cycle` with a step),
`switch` (the value and a start index in hidden bindings: the first
matching case, else the default's; every section from there on runs
inside a one-pass `Cycle`, so `break` leaves it), the ternary (a
`Choose`), `static` (a hidden global, its setting gathered while the
body is read and run where the function is defined), `global`,
`define`/`const`, `var_dump`, compound assignment, `break n` (an escape
carrying its count down through the cycles), hoisted top-level
functions and exponent literals. The reference kernels skip those
labels, so the reference suites under `tests/` run on this kernel and
stack8 only.

Classes are values here too: `Form::Class` builds one from a plan and the
values written for its members, and `Form::Attempt` holds a body, its
clauses and its last part. Because a return, a break and a raised value
are all escapes in this kernel, the last part runs on the way out of any
of them without being written twice.

It also holds a map: `Value::Dict`, keys with their values in the order
they were written, beside the plain vector. A literal gathers couples
into a dict and everything else into a vector, so a list stays a list;
writing a key a vector does not hold spreads that vector into a dict
first. `foreach` and `for v in a` are a `Cycle` over the places, reading
with `Extent`, `KeyAt` and `ItemAt`.
