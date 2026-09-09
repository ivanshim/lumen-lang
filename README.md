# Lumen-Lang

An experimental interpreter framework: one command-line host, six independent
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

# Choose the kernel explicitly (stack8, the fastest, is the default)
cargo run -- --kernel stream35 examples/lumen/pi_machin.lm
cargo run -- --kernel microcode11 examples/lumen/pi_machin.lm
cargo run -- --kernel microcode4 examples/lumen/pi_machin.lm
cargo run -- --kernel microcode7 examples/lumen/pi_machin.lm
cargo run -- --kernel stack5 examples/lumen/pi_machin.lm
cargo run -- --kernel stack8 examples/lumen/pi_machin.lm

# Write a program in another language (microcode11 only)
cargo run -- --kernel microcode11 --emit python examples/lumen/fibonacci_iterative.lm
cargo run -- --kernel microcode11 --emit rpl examples/python/demo.py
cargo run -- --kernel microcode11 --lang langs/extras/ruby.json --emit langs/extras/c.json examples/ruby/fibonacci.rb

# Name the language by name or by extension
cargo run -- --lang python examples/python/demo.py
cargo run -- --lang py examples/python/demo.py

# Reverse Polish Lumen: 5 3 + is 5 + 3, 8 'x' = assigns, « ... » is a program
cargo run -- examples/rplumen/fibonacci.rpl

# Run under a language definition read from disk at run time
cargo run -- --lang langs/php.json examples/php/loop.php
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

## Six kernels, one host

```
src/main.rs            the host: arguments, language detection, the embedded
                       Lumen standard library; the only place the kernels exist
kernels/stream35/      crate lumen-stream35: a tree-walking interpreter substrate
kernels/microcode11/   crate lumen-microcode11: the tree kept, and written back out
kernels/microcode4/    crate lumen-microcode4: four primitive forms and nothing else
kernels/microcode7/    crate lumen-microcode7: the four, and three forms for speed
kernels/stack5/        crate lumen-stack5: five words and nothing else
kernels/stack8/        crate lumen-stack8: the five, and three words for speed
langs/                 language definitions as JSON, one file per language,
                       with a generated side-by-side comparison; every kernel
                       reads them. Lumen, RPLumen, Python and Rust are embedded at
                       build time; langs/php.json and langs/extras/ are read
                       from disk at run time with --lang <path>
langs/lib_lumen/       the Lumen standard library, written in Lumen
langs/lib_<language>/  the same library as each other language spells it,
                       written by the porter from langs/lib_lumen/ and prepended
                       to every program in that language
examples/              programs for every language
```

The kernels are separate crates that do not depend on each other, and
`scripts/kernel_independence.py` fails CI if any names another or if a
long run of identical source lines appears in two trees. Where they need
the same facility they take different routes by design: comment removal
is a token-stream transformation in the stream35 language and a
definition-driven text pass in the others' scanners; bindings are hash-map
scopes in the stream35 kernel and slots resolved at compile time in the
five others, as frames up a chain in the tree kernels and as cells of a
flat frame in the stack kernels.

Each kernel is named for its shape and numbered by the size of its
instruction set: the number of node types, forms or words that everything
a definition spells is reduced to.

| Kernel | Product | Primitives |
|--------|---------|------------|
| `stream35` | a tree of handler nodes, one per construct | 35 node types |
| `microcode11` | a tree that keeps its source lines and is written back out by `--emit` | 11 forms |
| `microcode4` | a tree of four forms, everything else a call | 4 forms |
| `microcode7` | the four forms and a loop, a two-operand operator and a step: the fast tree | 7 forms |
| `stack5` | a flat word list of five kinds of word, everything else a shape made of them | 5 words |
| `stack8` | the five words and three fused from them: the fast kernel, and the default | 8 words |

### stream35, the stream kernel

A meta-language runtime. The kernel provides a lossless maximal-munch lexer,
a token registry, parser navigation, AST node traits, an execution loop and a
scoped environment. It knows no keyword, comment syntax, precedence, value
type or runtime policy. The language module gives each construct its meaning
in code, and reads the definition for everything else: which keyword,
operator, bracket and builtin name spells each construct, whether blocks are
indented or braced, what a comment or a variable looks like. The same four
definitions run here as on the other kernels, by a different method. See
[docs/LUMEN_KERNEL_STREAM35.md](docs/LUMEN_KERNEL_STREAM35.md).

### microcode11, the microcode kernel's second design

A four-stage pipeline (scan, structure, reduce, execute) that reads a
language as data, and the tree is the product: it keeps its source
lines, names are resolved to slots as it is built, and a postfix program
is read into it with a symbolic stack, so RPLumen runs as a tree with no
stack at all. What only a tree can do is be written back out: `--emit
<language>` prints any program in any language a definition describes,
driven by the target's definition read the other way round.
`scripts/translate_all.sh` writes every example in every language and
checks that each runs the same. See
[docs/LUMEN_KERNEL_MICROCODE11.md](docs/LUMEN_KERNEL_MICROCODE11.md).

### microcode4, the microcode kernel's third design

The floor. The tree has four forms, `Literal`, `Load`, `Assign` and
`Call`, and every construct of every language is a call: `a; b` is
`last(a, b)`, `if` is a call whose arms are program values, a loop is a
program that calls itself, a bare block is a program run at once,
`return`, `break` and `continue` are calls that unwind. Program values
are closures, calls in tail position replace the running program, and
each program says which exit it catches. All 488 programs print the
same as on the other kernels. See
[docs/LUMEN_KERNEL_MICROCODE4.md](docs/LUMEN_KERNEL_MICROCODE4.md).

### stack5, the stack kernel's second design

The stack machine at its floor. Five words: `Lit`, `Load`, `Store`,
`Apply` and `Unless`. A jump is `Lit false; Unless`, a loop is a jump
back, a call is `Apply` of the program on top of its arguments, a
function's result travels through a hidden slot and `return` jumps to the
end, `and` and `or` keep the left side in a hidden slot while they
decide, `dup` and `swap` are stores and loads of scratch slots, and an
array write takes the array out of its slot, rewrites it unshared and
stores it back. As fast as the first stack design's twenty-six words: the
other twenty-one bought no speed. All 498 programs print the same as on
the other kernels. See
[docs/LUMEN_KERNEL_STACK5.md](docs/LUMEN_KERNEL_STACK5.md).

### microcode7, the microcode kernel's fourth design

The four forms of microcode4 and three more that the kernel lab measured
worth a form of their own: `Cycle`, a loop run in the frame it appears
in; `Dyad`, an operator whose two operands are read without a visit to a
form; `Bump`, a binding stepped in place. A branch is still a call of
`choose` with program values for arms, which cost nothing once arms make
no frame. Two and a half times microcode4 on a bare loop, twice on an
arithmetic loop, and ahead of microcode11 everywhere. See
[docs/LUMEN_KERNEL_MICROCODE7.md](docs/LUMEN_KERNEL_MICROCODE7.md).

### stack8, the stack kernel's third design

The five words of stack5 and three fused from runs of them by a
peephole: `Dyad`, an operator whose operands come straight from cells
and never cross the stack; `Bump`, a cell stepped in place; `SkipCmp`, a
comparison with its conditional jump. Everything a language spells is
still first assembled from the five; the peephole then replaces each run
it recognises with one word. Five times stack5 on a bare loop, 1.6 times
on an arithmetic loop, and the fastest kernel on every program, so it is
the default. See [docs/LUMEN_KERNEL_STACK8.md](docs/LUMEN_KERNEL_STACK8.md).

## Languages

| Language | Extension | Definition | Style |
|----------|-----------|------------|-------|
| Lumen | `.lm` | built in | Python-style indentation, exact numbers, pipe operator |
| RPLumen | `.rpl` | built in | reverse Polish Lumen with Lumen's indented blocks: `5 3 +`, `8 'x' =`, `« 'n' = ... » 'f' =`, `cond if` / `else`, `while cond`, `0 10 'i' for`, `dup drop swap over rot` |
| Python | `.py` | built in | indentation with `:`, `elif`, `def`, `range()`, `str`, `arr.append(x)`, `s[i]` |
| Rust | `.rs` | built in | braces, `let mut`, `fn main()`, `println!("{}", x)`, `v.len()`, `x.to_string()` |
| PHP | `.php` | `langs/php.json` | braces, `$variables`, `<?php`, case-insensitive keywords, `strval`, `array_push` |
| Ruby | `.rb` | `langs/extras/ruby.json` | keyword blocks closed by `end`, `elsif`, `def`, `puts`, `nil`, `x.to_s`, `s.length` |
| Pascal | `.pas` | `langs/extras/pascal.json` | `begin`/`end`, `:=`, `<>`, `div`/`mod`, `function f(n: integer): integer;` with a `var` section, `f := ...` |
| C | `.c` | `langs/extras/c.json` | braces, `int x = 0;`, `long fib(int n)`, `printf("%d\n", x)`, `puts`, `main` |
| JavaScript | `.js` | `langs/extras/javascript.json` | braces, `let`/`const`, `function`, `===`, `**`, `console.log`, `arr.push(x)`, `s.length` |
| Swift | `.swift` | `langs/extras/swift.json` | braces without `;`, `let`/`var x: Int`, `func f(n: Int) -> Int`, `f(n: 1)`, `0..<n`, `arr.append(x)` |

The nine other languages are subsets of those languages spelled exactly as
the languages spell them, running on every kernel with Lumen's semantics: one value model,
one scoping rule, and `/` yielding an exact rational in Lumen or a real
where the language says so (`op.div.result`). Constructs the kernel lacks
(maps, `foreach`, `echo`, C's pointers, Swift's optionals) are left out of
their definitions rather than approximated. The built-in definitions are
compiled into the binary; PHP and the ones in `langs/extras/` are read
from disk with `--lang <path>`, which is also how a definition of your
own is run.

Every Lumen example is also written in every other language whose
definition spells what it needs: `scripts/port_examples.py` reads
`examples/lumen/`, ports each program into `examples/<language>/` under
the same relative path, and writes [examples/PORTS.md](examples/PORTS.md),
which says for each example and language either that the port exists or
which construct the language has no spelling for. Those ports, the
hand-written examples and the Lumen suite all run on every kernel, and
the suite requires the six kernels to print the same for every one of
them.

The library goes with them. The same porter writes every function of
`langs/lib_lumen/` that a language can spell into `langs/lib_<language>/`, file for
file, and the host prepends that mirror to every program in the language,
as it prepends `langs/lib_lumen/` to every Lumen program; so a Python or Pascal
program calls `gcd` or `substring` as a Lumen program does, and a port
carries no library code of its own. [docs/LIBRARY_PORTS.md](docs/LIBRARY_PORTS.md)
says for each function and language whether the mirror has it, or which
construct stands in the way, or which builtin the language spells it
with instead. A mirror costs a program a few milliseconds to read;
`LUMEN_BARE=1` runs a program without its library.

Lumen is the reference language: integers, exact rationals and reals of
configurable precision, strings, arrays, functions, `for`/`while`/`until`
loops, a pipe operator, base-N literals, and a standard library in
`langs/lib_lumen/` built on a handful of kernel built-ins. Its design principles are
in [docs/LUMEN_LANGUAGE_DESIGN.md](docs/LUMEN_LANGUAGE_DESIGN.md), and every
label of its definition is compared with the other languages in
[langs/README.md](langs/README.md).

## Testing

Every example runs on every kernel:

```bash
./test.sh --lang all          # everything
./test.sh                     # Lumen only
./test.sh --lang php          # one language
./test.sh --kernel stack8       # one kernel
./test.sh fibonacci_iterative.lm
./test.sh --help
```

The suite is also the differential test: each program runs on stream35
first, and every other kernel must print what it printed, whether the
program succeeds or fails; today every program does. The differential
test exists to find semantic gaps between the implementations, and each
one it has found has been closed in whichever kernel was wrong.
`scripts/kernel_diff.sh` runs the suite over every language. The suite
uses the release binary; the debug one is ten times slower on the heavy
programs, and the whole run takes about a minute.

GitHub Actions runs the independence check, the build with warnings as
errors, a check that the ported examples match what
`scripts/port_examples.py` writes, and the whole suite on every push.
`TEST_QUIET=1` prints program output only for failures.

## Reference suites

`tests/` holds tests the languages' own projects wrote: php-src's
`tests/lang`, `tests/basic` and `tests/func`, and the core-language files
of CPython's `Lib/test`, copied unchanged with their licenses.
`scripts/reference_tests.py` runs them against the definitions and writes
[tests/REPORT.md](tests/REPORT.md): what passes, why the rest does not,
which reserved words the definition spells and which functions the suites
call that it does not. The suites run on the two full kernels, stack8 and
microcode7, the ones that read the `ext.` labels a language needs beyond
the 133 core labels, and the only two that hold a map value: PHP's
associative arrays and Python's dictionaries, with `foreach` and
`print_r`. The report is the measure of the distance and the order to
close it in. See [tests/README.md](tests/README.md).

## The web

A Lumen program can answer web requests. The host gathers a request the
way a web server has always handed one to a program — the parts of it in
the environment, the body on the input — and the full kernels bind them
under whatever the language calls them, `$_GET`, `$_POST`, `$_COOKIE`,
`$_SERVER` and `$_REQUEST` for PHP.

```bash
lumen-lang --serve 8080 --lang langs/php.json site.php
curl 'http://127.0.0.1:8080/hello?who=Ada'
```

`--serve` answers each request by running the program once with that
request in its environment, so a served run is an ordinary run. A
program may write headers before a blank line, as CGI has always let it;
what follows is the body. Run without `--serve`, the same program reads
whatever request the environment holds, so it works behind any web
server that speaks CGI.

## The kernel lab

stack8 and microcode7 came out of an experiment recorded in
[docs/KERNEL_LAB.md](docs/KERNEL_LAB.md): copies of the floor kernels
stack5 and microcode4 were patched cycle by cycle for speed and measured
after each with `scripts/bench.sh` over the programs in `bench/`. The
notebook records every cycle, the predictions and the results, a second
experiment that separated what the new primitives bought from what the
engineering around them bought (on the stack machine nearly all of it,
on the tree about half; those improvements are folded into stack5 and
microcode4), and an ablation of every added primitive, which settled the
counts at eight and seven. The survivors were then rewritten in their own
words as kernels and the specimens removed; the first designs of each
shape, stack26 and microcode10, were retired at the same time.

## Documentation

- [docs/LUMEN_KERNEL_STREAM35.md](docs/LUMEN_KERNEL_STREAM35.md) — the stream35 kernel's charter
- [docs/LUMEN_KERNEL_MICROCODE11.md](docs/LUMEN_KERNEL_MICROCODE11.md) — the microcode11 kernel: the tree kept and written back out
- [docs/LUMEN_KERNEL_MICROCODE4.md](docs/LUMEN_KERNEL_MICROCODE4.md) — the microcode4 kernel: four primitive forms
- [docs/LUMEN_KERNEL_STACK5.md](docs/LUMEN_KERNEL_STACK5.md) — the stack5 kernel: five words and the shapes made of them
- [docs/LUMEN_KERNEL_MICROCODE7.md](docs/LUMEN_KERNEL_MICROCODE7.md) — the microcode7 kernel: the fast tree, seven forms
- [docs/LUMEN_KERNEL_STACK8.md](docs/LUMEN_KERNEL_STACK8.md) — the stack8 kernel: the fast stack machine, eight words
- [docs/KERNEL_LAB.md](docs/KERNEL_LAB.md) — the kernel lab: evolving both kernel shapes for speed, cycle by cycle
- [langs/README.md](langs/README.md) — the definition format, every label, and the languages side by side
- [docs/REFERENCE_SUITE_WORK.md](docs/REFERENCE_SUITE_WORK.md) — working on the reference test suite: what to run, where a label goes, and what goes wrong quietly
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
