# Stack Kernel

`kernels/stack/` compiles every language to one stack machine and runs
it. Where the stream kernel walks a tree of handlers and the microcode
kernel reduces to a tree of seven primitives, this kernel builds no tree:
a single pass over the tokens emits a flat list of words, and the machine
runs the words in a loop over one data stack. RPLumen is the machine's
own notation, so an RPLumen program compiles almost word for word; the
infix languages become RPLumen underneath.

Nothing in `kernels/stack/src/` names a keyword, an operator, a comment
marker or a function; all of that comes from a definition in `langs/`.

## The stages

| Stage | File | What it does | What the definition supplies |
|---|---|---|---|
| Scan | `lexer.rs` | text → tokens: words, numbers, strings (unescaped), quoted names, symbols, line ends, indentation | comment markers, prologue, string quotes and escapes, number punctuation, the identifier class, variable prefix, case folding, builtin names read whole, the name quote |
| Layout | `layout.rs` | tokens → tokens with block delimiters; line ends inside brackets dropped | block style, indent size, bracket pairs |
| Compile | `compiler.rs` | tokens → words, in one pass; names → slots | every keyword, operator tier, bracket, literal word, builtin name and statement form; the stack words and program delimiters of a postfix language |
| Run | `machine.rs` | words → effects | the words print renders booleans and null with, the print placeholders, whether strings index, whether a function's result is what it assigned to its own name, the system names |

Supporting modules: `code.rs` (the word set), `value.rs` (values and
rendering), `number.rs` (the numeric tower), `definition.rs` (the strict
reader).

## The word set

```
Lit v            push a constant            Jump t           go to t
Load p           push a binding             Unless t         pop; go to t when false
Store p          pop into a binding         When t           pop; go to t when true
PutAt p          pop value, index; write    Return           pop the result and leave
Append p         pop value; append          Exit             leave with the result so far
Forget           drop a bare block's names  Result           pop; remember as the result
Op o             pop operands; push result  Call p n         pop n arguments; run; push
Truth            pop; push its truth        Apply b n        pop n arguments; builtin; push
Dup Drop Swap Over Rot                      Eval             pop a program; run it
Mark / Gather    array literal (postfix)    Run p            run the program bound, or push
Collect n        pop n into an array
```

`Op` carries the floor and what is derived from it: `+ - * // /`,
equality and less-than are computed; `%`, `**`, negation and the other
comparisons are derived at the operation site the same way the other
kernels derive them. `Apply` reaches the builtins by the enum the
definition mapped the surface names onto.

## How constructs compile

- **Expressions** come out in post-order: `a * (b + c)` is `Load a, Load
  b, Load c, Op Add, Op Mul`. Precedence and brackets exist only in the
  compiler.
- **`and` / `or`** short-circuit as branches: `Truth, Dup, Unless L, Drop,
  <right>, Truth, L:`.
- **`if`** is `<cond>, Unless else, <then>, Jump end, else: <else>, end:`;
  a keyword-style chain shares one closer as the source does.
- **`while`** is `top: <cond>, Unless end, <body>, Jump top, end:`;
  `until` compiles its condition first, lifts the words out and puts them
  back after the body; `for` stores the bound in a hidden slot and counts
  with `Lt` and `Add`. `break` and `continue` are jumps patched when the
  loop closes; `continue` in a `for` goes to the step.
- **Functions** are programs: a `Lit` of the compiled body followed by a
  `Store` under the name. Parameters are the first slots. A call is
  `<args>, Call name n`; the callee finds the program by name, youngest
  binding first, so Pascal's result variable can shadow the function it
  is inside. A function's result is what it returned, else the value of
  its last expression statement, else, where the language says so, what
  it assigned to its own name.
- **Assignment** compiles the target as a load and then turns that load
  into a store; an indexed target becomes `PutAt`. `push(arr, v)` and
  `put(arr, i, v)`, in call and in method form, name the array rather than
  evaluating it.
- **Postfix** (RPLumen) has no expressions: each word is emitted as it is
  read. A quoted name is taken by the word after it (`=`, `for`, `push`,
  `put`); `« ... »` compiles its body as a program and pushes it; `[ ... ]`
  is `Mark`, the body, `Gather`.

## Names

Names are resolved when compiling. Inside a function every name it assigns
is a local slot; every other name is a global slot. A `Load` names both,
and a local slot that holds nothing yet falls through to the global, which
gives a callee the view of its caller's world that the other kernels give
through their frame chains. A bare block gets fresh slots for what it
binds and a `Forget` on the way out, so it shadows rather than overwrites.
At the top level everything is global; the system bindings (`ARGS`,
`MEMOIZATION`, the kind names) are globals seeded before the program runs.

## Values and speed

Integers that fit a machine word are unboxed and take checked native
arithmetic; anything else is a reference-counted big integer, fraction or
real. Strings, arrays and programs are reference-counted, so a value moves
on and off the stack as a pointer, and an array copies only when written
through a shared reference. That, and the absence of a tree, is where the
speed comes from: on a 300,000-iteration arithmetic loop the stack kernel
runs about five times faster than the microcode kernel and thirty times
faster than the stream kernel; on the sieve, which reads arrays in a loop,
forty times faster than the microcode kernel. Programs bound by big-integer
arithmetic (`pi_machin`) run at the same speed on every kernel.

## Relationship to the other kernels

The kernels never import each other and meet only in the host.
`scripts/kernel_independence.py` checks every pair for references and for
copied runs of source. `scripts/kernel_diff.sh` compares the stream and
stack kernels with the microcode kernel on every program under `examples/`;
today all 488 print the same. Because RPLumen is the machine's notation, a
flag that prints any program as the words it compiles to would print
RPLumen; that flag does not exist yet.
