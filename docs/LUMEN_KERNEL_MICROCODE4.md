# Microcode Kernel, Third Design (microcode4)

`kernels/microcode4/` is the microcode idea taken to its floor. The tree
has four forms and nothing else:

```
Literal   a constant; a program among them
Load      a binding
Assign    a binding written
Call      an operation of the kernel, or a program value, applied
```

Every construct of every language reduces to those four. The nine forms
of the second design that are missing here are all calls:

| Construct | Reduces to |
|---|---|
| `a; b` | `last(a, b)`: an operation returning its last argument, arguments evaluated in order |
| `if c A else B` | `if(c, «A», «B»)`: the arms are program values, so only one runs |
| `while c { body }` | `loop = « if(c, « body(); loop() », « null ») »; loop()`: a program that calls itself |
| `until c { body }` | `loop = « body(); if(c, « null », « loop() ») »; loop()` |
| `for v in a..b { body }` | `end = b; v = a; loop = « if(v < end, « body(); v = v + 1; loop() », « null ») »; loop()` |
| `{ ... }` (a bare block) | `« ... »()`: a program run at once |
| `a and b`, `a or b` | `and(a, «b»)`, `or(a, «b»)`: the right side a program, run only if needed |
| `return v`, `break`, `continue` | `return(v)`, `break()`, `continue()`: calls that unwind to the program that catches them |
| `x[i] = v`, `push(x, v)` | `put(x, i, v)`, `push(x, v)` |
| `f(a, b)`, `-x`, `a + b`, `[a, b]` | calls of the program `f`, of `neg`, of `add`, of `array` |

Nothing in `kernels/microcode4/src/` names a keyword, an operator, a
comment marker or a function; every spelling comes from a definition.

## The stages

| Stage | File | What it does |
|---|---|---|
| Lex | `lexer.rs` | text to tokens |
| Blocks | `blocks.rs` | indentation to block tokens; line ends inside brackets dropped |
| Reduce | `reduce.rs` | tokens to the four forms; names resolved to frames and slots; RPLumen read with a symbolic stack |
| Run | `run.rs` | the four forms evaluated, with tail calls |

Supporting: `tree.rs` (the forms), `spec.rs` (the definition as data),
`value.rs`, `arith.rs`.

## What holds it up

Three mechanisms in the executor make the four forms enough. None is a
form; each is how a form behaves.

- **Program values are closures.** A `Literal` holding a program
  evaluates to that program bound to the frame it was evaluated in, and
  calling it makes a frame under that one. So the arms of an `if`, the
  body of a loop, and the loop program itself see the bindings of the
  program around them, at a depth the reducer computed. The reducer
  keeps one scope per program: functions and bare blocks own names;
  arms and loop bodies own none and resolve into the program around them.
  A program that owns no names makes no frame either: it runs in the
  frame it closed over, and the reducer counts depth over frame-making
  scopes only. A binding is addressed as (frames up, slot); an empty
  slot falls through to the global of the same name, as in the other
  kernels.
- **Tail calls.** The executor runs a program body in tail position:
  `last` hands its last argument on, `if` hands the chosen arm on, and a
  call of a program value in that position replaces the running program
  in the same native frame instead of nesting. That is why a loop written
  as a program that calls itself runs three million iterations in
  constant stack. What the replaced programs caught is still caught.
- **Signals.** `return`, `break` and `continue` travel up as the error
  side of a result. Each program says what it catches: a function
  catches return, the program a loop is made of catches break, a loop's
  body catches continue, an arm catches nothing.

## Cost

Every `if` makes two closures and calls one; every loop iteration calls
three programs, none of which makes a frame. A call evaluates its
arguments straight into the callee's slots, an operator's into a fixed
buffer of three, and two machine integers under an operator are computed
in place: the kernel lab's count-neutral findings, folded back in and
worth about 2 times on loops and calls (`docs/KERNEL_LAB.md`). Before
the fold, against the second design, which walks its nine forms
directly, this design took about 1.6 times the time on an arithmetic
loop; after it, four forms are ahead of nine on loops and calls and
level everywhere else, and well ahead of microcode10, because the value
model is the fast one: unboxed machine integers, reference-counted
arrays copied on write.

Best of five, release build, seconds, after the fold:

| Program | microcode10 | stack26 | microcode11 | microcode4 | stack5 |
|---|---|---|---|---|---|
| loop | 0.309 | 0.072 | 0.187 | 0.127 | 0.055 |
| loop3m | 0.774 | 0.301 | 0.785 | 0.592 | 0.224 |
| fib | 0.090 | 0.034 | 0.068 | 0.048 | 0.032 |
| sieve | 1.089 | 0.019 | 0.029 | 0.029 | 0.020 |
| strings | 0.052 | 0.039 | 0.046 | 0.047 | 0.040 |
| pi | 0.862 | 0.850 | 0.864 | 0.861 | 0.856 |

What is lost against the second design is legibility and the emitter:
the tree no longer says "this is a loop", it says "this program calls
itself", and writing it back out in another language would mean
recognising that. This kernel exists to show the floor, not to be
written back from.

## Relationship to the other kernels

The kernels never import each other; `scripts/kernel_independence.py`
checks every pair. All 488 example programs print the same on this
kernel as on the other four.
