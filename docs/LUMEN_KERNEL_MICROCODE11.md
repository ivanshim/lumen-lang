# Microcode Kernel, Second Design (microcode11)

`kernels/microcode11/` keeps the microcode idea, four stages that turn a
language read as data into a small tree of primitives and then run it,
and changes what the tree is for. In the first design the tree is a step
on the way to running. Here it is the product: it keeps its source lines,
it is built with names already resolved, a postfix program is read into
it with a symbolic stack, and it can be written back out in any language
a definition describes. That last ability is the reason the kernel exists.

Nothing in `kernels/microcode11/src/` names a keyword, an operator, a
comment marker or a function; every spelling comes from a definition in
`langs/`, in both directions.

## The four stages, and the fifth

| Stage | File | Input → output | What the definition supplies |
|---|---|---|---|
| 1. Ingest | `ingest.rs` | text → tokens | comment markers, prologue, string quotes and escapes, number punctuation, identifier class, variable prefix, case folding, builtin names read whole, the name quote |
| 2. Structure | `structure.rs` | tokens → tokens with block delimiters | block style, indent size, bracket pairs |
| 3. Reduce | `reduce.rs` | tokens → the tree, names resolved to slots | every keyword, operator tier, bracket, literal word, builtin name and statement form; the stack words of a postfix language |
| 4. Execute | `execute.rs` | tree → values | the words print renders with, print placeholders, whether strings index, result-by-name, the system names |
| 5. Emit | `emit.rs` | tree → text in another language | the target's definition, read the other way round |

Supporting modules: `tree.rs` (the forms), `spec.rs` (the definition as a
table of labels with shapes), `value.rs`, `numeric.rs`.

## The tree

Nine forms, and two leaves:

```
Sequence      in order; the value is the last one's
Scope         a body whose new bindings are forgotten afterwards
Branch        test, then, otherwise
Loop          while test: body, then step (also after continue)
Assign        bind a name
AssignIndex   write an element of a named array
Call          a builtin or a program, by name or as a value
Operate       an operation on operands
Leave         return, break, continue
Literal       a constant, programs among them
Load          a binding
```

Every node carries its source line. A `for` is a `Sequence` of the bound
into a hidden slot, the variable's start, and a `Loop` whose step adds
one; `until` is a `Loop` on true whose step breaks on the condition; a
function definition is an `Assign` of a `Literal` program. The emitter
recognises those shapes and writes them back as `for` and `until`.

## Names

Names are resolved when the tree is built, as in the stack26 kernel: inside
a program every name it assigns is a local slot and every other name is
a global; a `Load` names both, and an empty local slot falls through to
the global of the same name. A `Slot` keeps the name, so the emitter can
write it and the executor can report it. There are no hidden names in
the environment: a loop bound lives in a slot that has no name.

## Reading a postfix language

RPLumen is reduced with a symbolic stack. A literal or a load pushes a
node; an operator pops two nodes and pushes an `Operate`; `'x' =` pops a
node into an `Assign`; `print` pops a node into a `Call` statement; `if`
pops its test and reads its bodies with fresh stacks. So `5 3 +` and
`5 + 3` become the same node, and an RPLumen program runs as a tree with
no stack at all, at the speed of the Lumen program it is.

Two consequences follow. A program's parameters are the values it pops
from an empty stack (`« 'b' = 'a' = ... »` takes two, `a` the deeper),
and what it leaves on the stack at the end is its result, so `eval` and a
call by name are ordinary calls. And the stack must balance statically:
the branches of an `if` must leave the same number of values, a loop body
must leave none, and a body may leave at most one. A program that
depends on the stack's depth at run time is not a tree and is refused
with the line. The arities of programs called before they are defined
(recursion) are found by reading the file leniently until the assumed
arities equal the found ones, then once more strictly.

A node with an effect (a call) that is still on the symbolic stack when a
statement is emitted is moved into a hidden slot first, so effects happen
in the order the stack machine would produce them.

## Writing a program out

`lumen-lang --kernel microcode11 --emit <language> <file>` reads the file
in its language and prints it in another, driven by the target's
definition alone:

- Expressions are written with the target's operator lexemes, and
  bracketed where the target's tiers require it. `and`/`or` stay
  operators; in a postfix target they become branches, since a stack has
  no short circuit.
- Statements take the target's block style: indentation with an intro
  token, braces, keyword blocks, or postfix control words. `elif` chains
  use `stmt.elif` where it exists, `else if` elsewhere, one closer for a
  keyword-style chain.
- Names are renamed where they clash with the target's reserved words or
  builtins, and take the target's variable prefix. The source's prefix
  is dropped.
- Builtins go by label: a source `len` is the target's `builtin.len`,
  whatever it is called. Where the target has no builtin but its
  definition's `$library` names a library function that provides the
  label, the call goes to that function. Where the source's `$library`
  says a function of its own provides a label the target has, the call
  becomes the target's builtin and the function is not written.
- `print` and `write` derive from each other and from `emit` the way the
  library does; a template call such as C's `printf("%d\n", x)` keeps
  its placeholders in a target that has them and becomes joined text
  elsewhere.
- Only the functions the program reaches are written; a library
  constant nothing reads is left out. A source entry function (C's
  `main`) joins the top level; a target with an entry gets one.
- A construct the target has no spelling for stops the whole program,
  with the reason: `no emit`, `a real with no finite decimal form`,
  `a program as a value`.

What comes out is the program as the target's definition describes the
language, which the kernels accept. Where a real compiler would want
what the definitions do not carry, nothing is invented: a C function
takes the first type word (`int`) for every parameter, a Pascal
declaration carries no type, a Rust `let` is `mut` when the name is
reassigned. `scripts/translate_all.sh` measures this: every example, in
every language, written in every language, run on the stack26 kernel and
compared with the original.

## Relationship to the other kernels

The kernels never import each other; `scripts/kernel_independence.py`
checks every pair. This kernel is the only one that can write a program,
because it is the only one whose product is a tree with names and lines
still in it: the stream35 kernel's product is a tree of closures and the
stack26 kernel's a word list. It runs the Lumen loop benchmark at about
twice the stack26 kernel's time and the same value model, and RPLumen at
the same speed as Lumen, since the stack was read away.
