# Microcode Kernel, Third Design

`kernels/microcode3/` is the microcode idea taken to its floor. The tree
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

Nothing in `kernels/microcode3/src/` names a keyword, an operator, a
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
  A binding is addressed as (frames up, slot); an empty slot falls
  through to the global of the same name, as in the other kernels.
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

Every `if` makes two closures and calls one; every loop iteration makes
three frames. Against the second design, which walks its nine forms
directly, that is about 1.6 times the time on an arithmetic loop and
about the same everywhere else, and still twice as fast as the first
microcode kernel, because the value model is the fast one: unboxed
machine integers, reference-counted arrays copied on write.

| Program | microcode | stack | microcode2 | microcode3 |
|---|---|---|---|---|
| 300k loop, Lumen | 0.37s | 0.09s | 0.19s | 0.30s |
| same loop, RPLumen | 1.01s | 0.08s | 0.19s | 0.29s |
| sieve | 1.04s | 0.02s | 0.03s | 0.05s |
| pi_machin | 0.96s | 0.96s | 0.96s | 0.97s |

What is lost against the second design is legibility and the emitter:
the tree no longer says "this is a loop", it says "this program calls
itself", and writing it back out in another language would mean
recognising that. This kernel exists to show the floor, not to be
written back from.

## Relationship to the other kernels

The kernels never import each other; `scripts/kernel_independence.py`
checks every pair. All 488 example programs print the same on this
kernel as on the other four.
