# Microcode Kernel

## Overview

The microcode kernel is an execution engine that owns every algorithm and
knows no language. A language is a JSON definition of tables
(`langs/<language>.json`); the kernel reads it and behaves accordingly.
Nothing in `kernels/microcode/src/` names a keyword, an operator, a comment
marker or a function.

**Invariant:** the kernel owns all algorithms; all language-specific behaviour
is table-driven. Definitions contain no executable logic, only data.

## The four stages

| Stage | File | Input → output | What the definition supplies |
|---|---|---|---|
| 1. Ingest | `kernel/_1_ingest.rs` | source text → tokens (words, numbers, decoded strings, operators, line ends, indentation) | line and block comment markers, a prologue to drop, string delimiters with their escape letters and which quotes are raw, operator lexemes, number punctuation and hexadecimal prefix, the identifier character class, a variable prefix, case folding of keywords or identifiers |
| 2. Structure | `kernel/_2_structure.rs` | tokens → tokens with explicit block delimiters | block style (indentation or braces), indent size, delimiters, optional block-intro token, bracket pairs that suspend line structure |
| 3. Reduce | `kernel/_3_reduce.rs` | tokens → instruction tree | literal words, operator precedence and associativity with the kernel operation each maps to, statement keywords with the form each introduces, call/group/array syntax |
| 4. Execute | `kernel/_4_execute.rs` | instruction tree → values | the surface names that reach built-ins, the names of system bindings |

Supporting modules: `kernel/instruction.rs` (the instruction set),
`kernel/value.rs` (the value model), `kernel/numeric.rs` (the exact numeric
tower), `kernel/env.rs` (a linear binding stack with frame markers, plus the call cache), `schema.rs` (the
table types and the strict reader that builds them from a definition).

## The instruction set

Seven primitives carry all control and data flow:

1. **Sequence** — execute in order; the value is the last one
2. **Scope** — execute inside a fresh binding scope
3. **Branch** — conditional execution
4. **Assign** — bind a name, or write an array element
5. **Invoke** — call a built-in or a function value
6. **Operate** — apply a kernel operation to operands
7. **Transfer** — return, break or continue

`Literal` and `Variable` are the leaves they operate on, and one internal
`Loop` (condition, body, optional step run after each pass, including after
`continue`) carries iteration. Everything else is desugared in Reduce:

| Source construct | Reduces to |
|---|---|
| `while c { b }` | `Loop { c, b }` |
| `until c { b }` | `Loop { true, b, step: Branch { c, Transfer(Break) } }` |
| `for v in r { b }` | bind the range once, bind `v` to its start, `Loop { v < end, b, step: v = v + 1 }` |
| `fn f(p) { b }` | `Assign f = Literal(function value)` |
| `a[i] = x` | `Assign` to an index target |
| `x \|> f(y)` | `Invoke f(x, y)` |
| `[a, b]` | `Operate(ArrayLiteral, [a, b])` |

Functions are values whose bodies are shared, not copied, between the binding
and each call.

## Kernel operations

Schemas map operator lexemes onto this fixed set: `add sub mul div quot rem
pow eq ne lt le gt ge and or not negate concat range index pipe`. Arithmetic
runs on one exact numeric tower: integers stay integers for closed
operations, other results are reduced rationals, and a real on either side
makes the result real with the left real's precision. Comparisons are exact
for every numeric kind.

## Built-ins

The kernel can provide `emit print_line write real int_to_string
real_to_string rational_to_string bool_to_string array_to_string
null_to_string kind_to_string len char_at ord chr error kind num den int frac
extern push range`. A definition decides which surface names reach them.
Lumen maps `emit` and the conversion primitives and writes `print` in Lumen
itself; Python, PHP and Rust map `print` and friends directly. A builtin name
may end in one operator character, which is how Rust's `println!` is a
name. `print` and `write` join their arguments with spaces; when the first
argument is a string holding `{}` placeholders and more arguments follow,
they fill the placeholders in order. Booleans and null render with the
first spelling the definition gives for them, so Python prints `True`.

## System bindings

A definition may name a read-only arguments binding (`ARGS`), a memoization
switch (`MEMOIZATION`; when the binding holds `true`, function results are
cached for the calls made while it is in effect), kind meta-values
(`INTEGER`, `RATIONAL`, …), the binding holding the default real precision
(`REAL_DEFAULT_PRECISION`) and an entry function (Rust's `main`) that runs
once the program body has defined it. The kernel supplies the values; the
definition supplies the names.

## Language definitions

The format, every label and a side-by-side comparison of the languages are
in [langs/README.md](../langs/README.md). The reader in `schema.rs` is
strict: every label must be present, every value must have the label's
type, unknown keys are rejected, keywords must be shaped like identifiers,
and a label the kernel does not implement must be empty. The four
definitions are embedded at build time; `lumen-lang --lang file.json`
reads one from disk instead, and the language name and file extensions the
host uses come from the definitions themselves.

Adding a language means adding a JSON file and one line in
`kernels/microcode/src/lib.rs` that embeds it. Changing an operator's
precedence, or which word means `null`, is a data edit with no code change.

## Relationship to the stream kernel

The two kernels never import each other; they are separate crates that meet
only in the host binary, and `scripts/kernel_independence.py` (run in CI)
fails the build if either names the other or if any run of five or more
identical source lines appears in both trees. They also solve the shared
problems differently on purpose: comments are a text-level pass driven by
the definition here and a token-stream transformation in the stream
language; bindings live in a linear frame stack here and in a stack of
hash-map scopes there. The stream kernel delegates meaning to language code
through handler traits; the microcode kernel takes meaning from tables. Both
read the same definitions, every example runs on both under `test.sh` and
CI, and `scripts/kernel_diff.sh` requires their output to agree.
