# Stack Kernel, Second Design (stack5)

`kernels/stack5/` is the stack machine taken to its floor. Where stack26
has a word for everything a language does, this kernel has five:

```
Lit v        push a constant
Load slot    push a binding's value
Store slot   pop into a binding
Apply op, n  pop n arguments, apply a kernel operation, push its result
Unless to    pop; when the value is not true, continue at `to`
```

Every one of stack26's other twenty-one words is a shape made of these.
Nothing in `kernels/stack5/src/` names a keyword, an operator, a comment
marker or a function; every spelling comes from a definition in `langs/`.

## The shapes

| Construct | Assembled as |
|---|---|
| a jump | `Lit false; Unless to` |
| `if c A else B` | `c; Unless else; A; Lit false; Unless end; else: B; end:` |
| `while c body` | `top: c; Unless out; body; Lit false; Unless top; out:` |
| `until c body` | `top: body; c; Unless top` |
| `for v in a..b body` | `a; Store v; b; Store #end; top: Load v; Load #end; Apply lt; Unless out; body; Load v; Lit 1; Apply add; Store v; Lit false; Unless top; out:` |
| `break`, `continue` | jumps to the loop's exit and to its step |
| `f(a, b)` | `a; b; Load f; Apply call, 3`: the program is the last argument |
| a function's result | `Lit null; Store #result` first; `Store #result` after each expression statement; `Load #result` last; `return v` is `v` and a jump to the end |
| `a and b` | `a; Store #t; Load #t; Unless skip; b; Store #t; skip: Load #t; Apply truth` |
| `a or b` | the same with `Apply not` before the `Unless` |
| `-x`, `not x`, `a + b`, `x[i]` | `Apply neg`, `Apply not`, `Apply add`, `Apply index` |
| `[a, b]` | `a; b; Apply array, 2` |
| `[ 1 2 3 ]` in RPLumen | `Lit mark; 1 2 3; Apply gather`: down to the mark, count unknown until run time |
| `arr[i] = v`, `push(arr, v)`, `4 'arr' push` | `i; v; Load arr taking; Apply put; Store arr` |
| `dup`, `drop`, `swap`, `over`, `rot` | stores and loads of scratch slots: `dup` is `Store #a; Load #a; Load #a`, `swap` is `Store #a; Store #b; Load #a; Load #b` |
| `eval`, a bare RPLumen word | `Apply eval`, `Load w; Apply run` (run it if it is a program, else it is the value) |
| a bare block's bindings, on leaving | `Lit empty; Store slot` for each |
| a function, a program value | `Lit <program>`, then `Store name` where it has one |

The hidden slots (`#result`, `#t`, `#a`, `#b`, `#c`, `#endN`) are names no
program can spell. They live in the same frames as the program's own
names: locals inside a function, globals at the top level.

## The taking load

One shape needs a detail. An array written through a shared reference is
copied first, so `i; v; Load arr; Apply put; Store arr` would copy the
whole array on every element write and the sieve would go quadratic. The
load before a `put` or `push` therefore *takes*: it moves the value out
of its slot and leaves a hole, so the array on the stack is the only
reference and `put` rewrites it in place. The store after it is addressed
like the load, every candidate slot and the global, and fills the hole
wherever it finds it. Nothing runs between the take and the store, so no
program ever sees a hole. It is a flag on `Load`, not a sixth word.

## The stages

| Stage | File | What it does |
|---|---|---|
| Scan | `scan.rs` | text to tokens |
| Shape | `shape.rs` | indentation to block tokens; line ends inside brackets dropped |
| Assemble | `assemble.rs` | tokens to the five words, in one pass; names to slots |
| Run | `machine.rs` | one loop, five match arms, one stack |

Supporting: `words.rs` (the five words, the operations, the program),
`language.rs` (the definition as data), `values.rs`, `numbers.rs`.

## Names

As in stack26: a local is a slot in the running frame, a global a slot in
one table; a load names every slot of that name from the innermost open
block outward, then the one outside every block, then the global, and
reads the first that holds a value; a store writes the innermost, or the
global for a top-level name outside every block. A slot a bare block
declared is found only from inside that block, so after the block the
outer binding is back. A function sees its own bindings and the
program's, never its caller's: an empty local falls through to the global
and nowhere else. In a language
that names a function's result after the function (Pascal), a call of the
function's own name inside its body is assembled straight to the global,
since the local of that name is the result being built.

## Cost

The five words carry the same value model as stack26, unboxed machine
integers and reference-counted arrays copied on write, so the shapes can
cost only their extra dispatches: two words for a jump instead of one, a
slot write instead of a register for each expression statement's value,
stores and loads where stack26 shuffled the stack directly.

Measured (release build, best of several runs):

| Program | microcode10 | stack26 | microcode11 | microcode4 | stack5 |
|---|---|---|---|---|---|
| 300k loop, Lumen | 0.28s | 0.07s | 0.16s | 0.24s | 0.06s |
| same loop, RPLumen | 0.88s | 0.07s | 0.16s | 0.24s | 0.07s |
| 3M loop, Lumen | | 0.29s | | | 0.27s |
| sieve | 0.93s | 0.02s | 0.03s | 0.04s | 0.02s |
| pi_machin | 0.83s | 0.81s | 0.81s | 0.82s | 0.83s |

The extra dispatches cost nothing measurable: five words run as fast as
twenty-six, and a little faster on the Lumen loops, where a smaller
dispatch and a result slot in place of a result register make up for the
two-word jumps. The twenty-one words stack26 has beyond these five buy it
no speed; they are spellings.

## Relationship to the other kernels

The kernels never import each other; `scripts/kernel_independence.py`
checks every pair. `scripts/kernel_diff.sh` compares this kernel with the
others on every program under `examples/`; all 488 print the same.
microcode4 is the same exercise on the tree: four forms there, five words
here, and the one extra is the conditional jump, which a flat list needs
because it has no program values to hand a branch to.
