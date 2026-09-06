# Lumen-Lang

An experimental interpreter framework: one command-line host, two independent
kernels, three surface syntaxes. Lumen exists to explore language semantics
and the separation between an execution substrate and the languages it hosts.
It is not a production language.

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

# PHP-like (braces, dollar-prefixed variables)
cargo run -- examples/php/demo.php

# Choose the kernel explicitly (microcode is the default)
cargo run -- --kernel stream examples/lumen/pi_machin.lm
cargo run -- --kernel microcode examples/lumen/pi_machin.lm

# Run under a language definition read from disk
cargo run -- --config configs/php.json examples/php/loop.php
```

The language is picked from the file extension (`.lm`, `.py`, `.php`, `.rs`)
or with `--lang lumen|python|php|rust`; each definition in `configs/` names
its own extensions. Arguments after the file are passed to the program.

## Two kernels, one host

```
src/main.rs            the host: arguments, language detection, the embedded
                       Lumen standard library; the only place both kernels exist
kernels/stream/        crate lumen-stream: a tree-walking interpreter substrate
kernels/microcode/     crate lumen-microcode: a table-driven execution engine
lib_lumen/             the Lumen standard library, written in Lumen
configs/               language definitions as JSON, one file per language,
                       with a generated side-by-side comparison
examples/              programs for all three languages
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
type or runtime policy. Each language module registers its lexemes, supplies
its own handler traits and precedence scale, strips its own comments, and
defines its own values. See [docs/LUMEN_KERNEL_STREAM.md](docs/LUMEN_KERNEL_STREAM.md).

### Microcode kernel

A four-stage pipeline (ingest → structure → reduce → execute) that reads a
language as data. Each language is a JSON definition in `configs/` mapping a
fixed set of labels to spellings: lexemes, block rules, literal words,
operator precedence mapped onto kernel operations, statement keywords mapped
onto statement forms, the surface names of built-ins, and the names of
system bindings. The kernel reduces every construct to seven primitives and
executes them. See [docs/LUMEN_KERNEL_MICROCODE.md](docs/LUMEN_KERNEL_MICROCODE.md)
and [configs/README.md](configs/README.md).

## Languages

| Language | Extension | Style | Stream | Microcode |
|----------|-----------|-------|--------|-----------|
| Lumen | `.lm` | Python-style indentation, exact numbers, pipe operator | yes | yes |
| Python | `.py` | indentation with `:`, `elif`, `def`, `range()` | no | yes |
| PHP | `.php` | braces, `$variables`, `<?php`, case-insensitive keywords | no | yes |
| Rust | `.rs` | braces, `let mut`, `fn main()`, `println!("{}", x)` | no | yes |

Python, PHP and Rust are subsets of those languages spelled exactly as the
languages spell them, running on Lumen's semantics: exact rationals, one
value model, one scoping rule. Constructs the kernel lacks (maps, `foreach`,
`echo`) are left out of their definitions rather than approximated.

Lumen is the reference language: integers, exact rationals and reals of
configurable precision, strings, arrays, functions, `for`/`while`/`until`
loops, a pipe operator, base-N literals, and a standard library in
`lib_lumen/` built on a handful of kernel built-ins. Its design principles are
in [docs/LUMEN_LANGUAGE_DESIGN.md](docs/LUMEN_LANGUAGE_DESIGN.md).

## Testing

Every Lumen example runs on both kernels; the other languages run on the
microcode kernel:

```bash
./test.sh --lang all          # everything
./test.sh                     # Lumen only
./test.sh --lang php          # one language
./test.sh --kernel stream     # one kernel
./test.sh fibonacci_iterative.lm
./test.sh --help
```

The same command runs in GitHub Actions on every push, with warnings treated
as errors. `TEST_QUIET=1` prints program output only for failures.

## Documentation

- [docs/LUMEN_KERNEL_STREAM.md](docs/LUMEN_KERNEL_STREAM.md) — stream kernel charter
- [docs/LUMEN_KERNEL_MICROCODE.md](docs/LUMEN_KERNEL_MICROCODE.md) — microcode kernel and how it reads a definition
- [docs/LUMEN_LANGUAGE_DESIGN.md](docs/LUMEN_LANGUAGE_DESIGN.md) — design principles
- [docs/LUMEN_LANGUAGE_BNF.md](docs/LUMEN_LANGUAGE_BNF.md) — Lumen grammar, with EBNF for all three languages in `grammar/`
- [docs/LUMEN_COMPACT_REFERENCE.md](docs/LUMEN_COMPACT_REFERENCE.md) — Lumen quick reference
- [docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md](docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md) — external function design
- [docs/LANGUAGE_COMPARISON.md](docs/LANGUAGE_COMPARISON.md) — the three syntaxes side by side
- [configs/README.md](configs/README.md) — language definitions as data, with every label compared across languages
- [docs/LUMEN_LANGUAGE_ROADMAP.md](docs/LUMEN_LANGUAGE_ROADMAP.md) — planned evolution
- [docs/DIRECTORY_STRUCTURE.txt](docs/DIRECTORY_STRUCTURE.txt) — file map
- [docs/VERSION_HISTORY.md](docs/VERSION_HISTORY.md) — release notes
- `yaml/` — the long-form language specifications the schemas were derived from

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
