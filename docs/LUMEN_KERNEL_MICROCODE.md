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
| 1. Ingest | `kernel/_1_ingest.rs` | source text → tokens (words, numbers, decoded strings, operators, line ends, indentation) | line and block comment markers, a prologue to drop, string delimiters with their escape letters and which quotes are raw, operator lexemes, number punctuation and hexadecimal prefix, the identifier character class, a variable prefix, case folding of keywords or identifiers, builtin names read whole (`println!`, `console.log`), the quote around a name given as data (RPLumen's `'x'`) |
| 2. Structure | `kernel/_2_structure.rs` | tokens → tokens with explicit block delimiters | block style (indentation, braces, keyword or postfix), indent size, paired open/close delimiters, optional block-intro tokens, bracket pairs that suspend line structure |
| 3. Reduce | `kernel/_3_reduce.rs` | tokens → instruction tree | literal words, operator precedence and associativity with the kernel operation each maps to, statement keywords with the form each introduces, block-intro words to skip and the closer that ends a keyword-style chain, call/group/array syntax, argument labels to drop, type-first declarations, declarations before a function body and the terminator between parameter groups, the pipe's lexeme (a bare name after it is a call); for a postfix language, the stack words and the program delimiters |
| 4. Execute | `kernel/_4_execute.rs` | instruction tree → values | the surface names that reach built-ins, the placeholders print fills, whether a function's result is what it assigned to its own name, whether strings may be indexed, the names of system bindings, `get` and `put` as indexing by function |

A postfix language (RPLumen) reaches the same instruction set through
the reducer's second entry point: the stack is an array bound to a hidden
name at the start of the program, a literal becomes an invocation that
pushes it, an operator pops its operands into hidden temporaries and
pushes the result, a control word takes its condition from a pop, a
program value (`« ... »`) is a function value of no parameters, and a
bare word is an invocation that runs the program bound to it or pushes
its value. Six internal invocations (`<push>`, `<pop>`, `<word>`,
`<eval>`, `<depth>`, `<gather>`) carry the stack mechanics; no definition
can spell them. The executor gains nothing else.

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
| `for v in a..b { b }` | bind the end once, bind `v` to the start, `Loop { v < end, b, step: v = v + 1 }`; the range is loop syntax, not a value |
| `fn f(p) { b }` | `Assign f = Literal(function value)` |
| `a[i] = x` | `Assign` to an index target |
| `x \|> f(y)` | `Invoke f(x, y)` |
| `[a, b]` | `Operate(ArrayLiteral, [a, b])` |

Functions are values whose bodies are shared, not copied, between the binding
and each call.

## Kernel operations

Schemas map operator lexemes onto this fixed set: `add sub mul div quot rem
pow eq ne lt le gt ge and or not negate concat range index pipe`. Of these,
`add sub mul quot div lt eq and or not concat index` are the floor;
`rem` is `a - b * (a // b)`, `pow` is multiplication by squaring, `negate`
is `0 - x`, and `ne gt le ge` are `not eq`, `lt` reversed, `not lt`
reversed and `not lt`, computed that way in `numeric.rs` and
`_4_execute.rs`. `range` is loop syntax that reduce turns into a counted
loop, and `pipe` a call. Arithmetic runs on one exact numeric tower:
integers stay integers for closed operations, other results are reduced
rationals, and a real on either side makes the result real with the left
real's precision. Comparisons are exact for every numeric kind.

## Built-ins

The kernel can provide `emit print_line write real precision to_string
to_int to_real len char_at ord chr error kind num den extern push`, and
`range` as the call that spells a for loop's range. A definition decides
which surface names reach them. Lumen maps `emit` and the primitives that
reach into a value (`kind num den precision`), derives `int` and `frac`
from `num` and `den`, and writes `print` and every renderer in Lumen itself
(`lib_lumen/render.lm`), so the kernel renders nothing for Lumen; Python,
PHP and Rust map `print` and friends directly, and `to_string`, `to_int`
and `to_real` are the one-name conversions of languages that have them
(`str`, `String`, `intval`). A builtin name begins like an identifier and
may go on with operator characters and further words (`println!`,
`console.log`). `print` and `write` join their arguments with spaces; when
the first argument is a string holding a placeholder from
`builtin.print.placeholder` and more arguments follow, they fill the
placeholders in order. Booleans and null render with the first spelling the
definition gives for them, so Python prints `True`.

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
