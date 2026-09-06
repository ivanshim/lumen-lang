# Lumen-Lang

An experimental interpreter framework: one command-line host, two independent
kernels, and languages defined as data. Lumen exists to explore language
semantics and the separation between an execution substrate and the languages
it hosts. It is not a production language.

## Quick start

Requirements: Git and a Rust toolchain (https://www.rust-lang.org/tools/install).

```bash
git clone https://github.com/ivanshim/lumen-lang.git
cd lumen-lang
cargo build
cargo run -- examples/lumen/constructs/loop.lm
```

```bash
# Lumen (Python-style indentation)
cargo run -- examples/lumen/fibonacci_iterative.lm

# Rust-like (braces and semicolons)
cargo run -- examples/rust/demo.rs

# Python-like (indentation)
cargo run -- examples/python/fibonacci.py

# Rust-like (braces, fn main, println!)
cargo run -- examples/rust/demo.rs

# Choose the kernel explicitly (microcode is the default)
cargo run -- --kernel stream examples/lumen/pi_machin.lm
cargo run -- --kernel microcode examples/lumen/pi_machin.lm

# Name the language by name or by extension
cargo run -- --lang python examples/python/demo.py
cargo run -- --lang py examples/python/demo.py

# Run under a language definition read from disk at run time
cargo run -- --lang langs/extras/php.json examples/php/loop.php
cargo run -- --lang langs/extras/ruby.json examples/ruby/demo.rb
cargo run -- --lang langs/extras/c.json examples/c/fibonacci.c
cargo run -- --lang langs/extras/swift.json examples/swift/demo.swift
```

The language is picked with `--lang` (or `--language`), which takes a
language name (`lumen`, `python`, `rust`), a file extension (`lm`, `py`,
`rs`), or the path of a definition file; without the flag it comes from
the file extension, which each definition in `langs/` declares for itself,
and Lumen is the default. Arguments after the file are passed to the
program.

## Two kernels, one host

```
src/main.rs            the host: arguments, language detection, the embedded
                       Lumen standard library; the only place both kernels exist
kernels/stream/        crate lumen-stream: a tree-walking interpreter substrate
kernels/microcode/     crate lumen-microcode: a table-driven execution engine
langs/                 language definitions as JSON, one file per language,
                       with a generated side-by-side comparison; both kernels
                       read them. Lumen, Python and Rust are embedded at
                       build time; langs/extras/ holds definitions read from
                       disk at run time with --lang <path>
lib_lumen/             the Lumen standard library, written in Lumen
examples/              programs for every language
```

The kernels are separate crates that do not depend on each other, and
`scripts/kernel_independence.py` fails CI if either names the other or if any
run of five or more identical source lines appears in both trees. Where both
need the same facility they take different routes by design: comment removal
is a token-stream transformation in the stream language and a
definition-driven text pass in the microcode ingest; bindings are hash-map scopes in one and a
linear frame stack in the other.

### Stream kernel

A meta-language runtime. The kernel provides a lossless maximal-munch lexer,
a token registry, parser navigation, AST node traits, an execution loop and a
scoped environment. It knows no keyword, comment syntax, precedence, value
type or runtime policy. The language module gives each construct its meaning
in code, and reads the definition for everything else: which keyword,
operator, bracket and builtin name spells each construct, whether blocks are
indented or braced, what a comment or a variable looks like. The same four
definitions run here as on the microcode kernel, by a different method. See
[docs/LUMEN_KERNEL_STREAM.md](docs/LUMEN_KERNEL_STREAM.md).

### Microcode kernel

A four-stage pipeline (ingest → structure → reduce → execute) that reads a
language as data. Each language is a JSON definition in `langs/` mapping a
fixed set of labels to spellings: lexemes, block rules, literal words,
operator precedence mapped onto kernel operations, statement keywords mapped
onto statement forms, the surface names of built-ins, and the names of
system bindings. The kernel reduces every construct to seven primitives and
executes them. See [docs/LUMEN_KERNEL_MICROCODE.md](docs/LUMEN_KERNEL_MICROCODE.md)
and [langs/README.md](langs/README.md).

## Languages

| Language | Extension | Definition | Style | Stream | Microcode |
|----------|-----------|------------|-------|--------|-----------|
| Lumen | `.lm` | built in | Python-style indentation, exact numbers, pipe operator | yes | yes |
| Python | `.py` | built in | indentation with `:`, `elif`, `def`, `range()` | yes | yes |
| Rust | `.rs` | built in | braces, `let mut`, `fn main()`, `println!("{}", x)` | yes | yes |
| PHP | `.php` | `langs/extras/php.json` | braces, `$variables`, `<?php`, case-insensitive keywords | yes | yes |
| Ruby | `.rb` | `langs/extras/ruby.json` | keyword blocks closed by `end`, `elsif`, `def`, `puts`, `nil` | yes | yes |
| Pascal | `.pas` | `langs/extras/pascal.json` | `begin`/`end`, `:=`, `<>`, `div`/`mod`, `;` terminators, case-insensitive | yes | yes |
| C | `.c` | `langs/extras/c.json` | braces, `int x = 0;`, `long fib(int n)`, `printf("%d\n", x)`, `puts`, `main` | yes | yes |
| JavaScript | `.js` | `langs/extras/javascript.json` | braces, `let`/`const`, `function`, `===`, `**`, `console.log` | yes | yes |
| Swift | `.swift` | `langs/extras/swift.json` | braces without `;`, `let`/`var x: Int`, `func f(n: Int) -> Int`, `f(n: 1)`, `0..<n` | yes | yes |

The eight other languages are subsets of those languages spelled exactly as
the languages spell them, running on Lumen's semantics: one value model,
one scoping rule, and `/` yielding an exact rational in Lumen or a real
where the language says so (`op.div.result`). Constructs the kernel lacks
(maps, `foreach`, `echo`, C's pointers, Swift's optionals) are left out of
their definitions rather than approximated. The built-in definitions are
compiled into the binary; the ones in `langs/extras/` are read from disk
with `--lang <path>`, which is also how a definition of your own is run.

Every Lumen example is also written in every other language whose
definition spells what it needs: `scripts/port_examples.py` reads
`examples/lumen/`, ports each program (with the library functions it
calls) into `examples/<language>/` under the same relative path, and
writes [examples/PORTS.md](examples/PORTS.md), which says for each example
and language either that the port exists or which construct the language
has no spelling for. Those ports, the hand-written examples and the Lumen
suite all run on both kernels, and `scripts/kernel_diff.sh` checks that
the two kernels print the same for every one of them.

Lumen is the reference language: integers, exact rationals and reals of
configurable precision, strings, arrays, functions, `for`/`while`/`until`
loops, a pipe operator, base-N literals, and a standard library in
`lib_lumen/` built on a handful of kernel built-ins. Its design principles are
in [docs/LUMEN_LANGUAGE_DESIGN.md](docs/LUMEN_LANGUAGE_DESIGN.md), and every
label of its definition is compared with the other languages in
[langs/README.md](langs/README.md).

## Testing

Every example runs on both kernels:

```bash
./test.sh --lang all          # everything
./test.sh                     # Lumen only
./test.sh --lang php          # one language
./test.sh --kernel stream     # one kernel
./test.sh fibonacci_iterative.lm
./test.sh --help
```

`scripts/kernel_diff.sh` goes further and requires the two kernels to print
the same thing for every program; today every program does. The
differential test exists to find semantic gaps between the two
implementations, and each one it has found has been closed in whichever
kernel was wrong.

GitHub Actions runs the independence check, the build with warnings as
errors, a check that the ported examples match what
`scripts/port_examples.py` writes, and the whole suite on every push.
`TEST_QUIET=1` prints program output only for failures.

## Documentation

- [docs/LUMEN_KERNEL_STREAM.md](docs/LUMEN_KERNEL_STREAM.md) — stream kernel charter
- [docs/LUMEN_KERNEL_MICROCODE.md](docs/LUMEN_KERNEL_MICROCODE.md) — microcode kernel and how it reads a definition
- [langs/README.md](langs/README.md) — the definition format, every label, and the languages side by side
- [docs/LUMEN_LANGUAGE_DESIGN.md](docs/LUMEN_LANGUAGE_DESIGN.md) — design principles
- [docs/LUMEN_COMPACT_REFERENCE.md](docs/LUMEN_COMPACT_REFERENCE.md) — Lumen quick reference
- [docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md](docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md) — external function design
- [docs/LUMEN_LANGUAGE_ROADMAP.md](docs/LUMEN_LANGUAGE_ROADMAP.md) — planned evolution
- [docs/DIRECTORY_STRUCTURE.txt](docs/DIRECTORY_STRUCTURE.txt) — file map
- [docs/VERSION_HISTORY.md](docs/VERSION_HISTORY.md) — release notes

## Philosophy

Lumen prioritises clarity over speed, small honest semantics over breadth,
and explicit behaviour over cleverness. The AST is the source of truth,
failures are loud, and the whole interpreter should stay inspectable by one
person in one sitting.

## License

Provided as-is for educational and experimental purposes.

## Attribution

Project lead: Ivan Shim. Implementation with AI assistance: GPT-5.2
(consulting), Claude Code Haiku 4.5 (v0.0.x), Claude Fable 5.1 (v0.2.0
cleanup and kernel-fidelity pass).
