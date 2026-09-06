# Lumen Version History

This document records each public milestone of Lumen in chronological order.
Each entry is intentionally self-contained so that it remains meaningful even if surrounding detail is trimmed in the future.

---

## Unreleased
**Contributors:** Ivan Shim orchestrating, Claude Fable 5.1 coding
**Change:** Languages defined as data; both kernels read the definitions

### What was done:
- **Definitions**: `langs/<language>.json` is the definition of a
  language's surface: a fixed set of labels, each mapped to the strings the
  language spells it with. Lexeme labels take alias lists, settings take
  scalars, precedence is a list of tiers. `langs/README.md` states the
  format rules and carries a generated table comparing every label across
  every language. Three block styles: indentation, paired braces (`{ }`,
  `begin`/`end`) and keyword blocks closed by one word (`then` ... `end`);
  several block-comment pairs per language; a binding with an annotation
  and no value binds null. Type-first declarations (`stmt.let.type_first`,
  C's `int x = 0;` and `long fib(int n)`), argument labels at call sites
  (`syntax.call.label`, Swift's `fib(n: 10)`), print placeholders as data
  (`builtin.print.placeholder`, Rust's `{}` and C's `%d`), and builtin
  names with operator characters inside (`console.log`).
- **Extra languages**: `langs/extras/` holds definitions that are not
  compiled in and are read from disk with `--lang <path>`, as proof that a
  definition is loaded at run time. PHP moved there, and Ruby, Pascal, C,
  JavaScript and Swift are added with examples; all nine languages run on
  both kernels.
- **Division as the languages specify it**: `op.div.result` says whether a
  language's `/` yields an exact rational (Lumen) or a real (Python,
  JavaScript, PHP, Pascal), so `7 / 2` prints `3.5` where the language
  says so. A binding with no value binds null in every language that
  declares (`let x;`).
- **The Lumen suite in every language**: `scripts/port_examples.py` ports
  each of the 84 Lumen examples, with the library functions it calls, to
  every other language whose definition spells what it needs, 306
  programs in all, and writes `examples/PORTS.md` saying which and, where
  not, which construct the language has no spelling for. Builtins are
  written the way each language writes them (`arr.push(x)`, `s.length`,
  `x.to_s`, `strlen`), a constant a function reads is inlined where
  functions cannot see top-level names, and Pascal gets typed functions
  with `var` sections. The test driver runs every example under
  `examples/` on both kernels (842 runs), and `scripts/kernel_diff.sh`
  finds all 421 programs printing the same on both.
- **The kernel renders nothing for Lumen**: the seven renderer builtins
  (`int_to_string` and the rest) are gone from both kernels and live in
  `lib_lumen/render.lm` as Lumen code, written against the primitives; a
  new primitive, `precision`, reads the significant digits a real
  carries. `to_string`, `to_int` and `to_real` are the one-name
  conversions of languages that have them (`str`, `String`, `intval`);
  Lumen gives them no name. Every Lumen example prints exactly what it did.
- **The floor**: every operation the kernels can express in terms of the
  others is now derived, in both kernels the same way, and the definitions
  README states the floor per domain. `%` is `a - b * (a // b)`, `**` is
  multiplication by squaring, `-x` is `0 - x`, and `!=`, `>`, `<=`, `>=`
  come from `==` and `<`. `num` and `den` read the fraction any number is,
  so `int` and `frac` are library code (`lib_lumen/numeric.lm`). A range
  is loop syntax reduced to a counted loop, not a value: the `Range` kind
  and the range operator outside a `for` are gone. `lib_lumen/array.lm`
  derives concatenation, slicing, search and reversal from `len`, indexing
  and `push`. Python's `sys.stdout.write` and JavaScript's
  `process.stdout.write` are `emit`, the string-only writer, and their
  `write` is derived as Lumen's is, `emit(to_string(x))`. Every Lumen
  example prints exactly what it did; 310 ports, 852 runs, 426 programs
  identical on both kernels.
- **Method syntax and Pascal functions**: the pipe may be spelled `.` at
  the highest tier, and a bare name after it is a call, so `arr.push(x)`,
  `s.length`, `42.to_s` and `v.len()` are the same kernel calls as
  `push(arr, x)`, `len(s)`, `to_string(42)`; `op.index.strings` lets
  `s[i]` index a string. A function header ending in a terminator may be
  followed by declarations before the body, typed parameter groups are
  separated by the terminator, and `stmt.function.result_by_name` makes
  the value assigned to the function's own name its result, which is how
  Pascal declares `function f(n: integer): integer;`.
- **Kernels agree**: `scripts/kernel_diff.sh` reports every example
  printing the same on both kernels. The stream kernel's integer quotient
  now divides the exact values before truncating, its reals keep zero
  digits after the point (`2.05`, `0.01`), a real between -1 and 0 keeps
  its sign on both kernels, `+` with a string on one side concatenates as
  the microcode does, and the microcode's `extern` capabilities follow
  `docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md` (`print_native` returns the value,
  `value_type` a number code) with equality exact across numeric kinds.
- **Microcode kernel**: reads the definitions instead of YAML schemas, with a
  strict reader (every label present, right type, no unknown keys, keywords
  shaped like identifiers, unimplemented labels empty). Gains block comments,
  a prologue, a variable prefix, hexadecimal literals, per-quote escape
  rules, case-folded keywords, builtin names ending in an operator character
  (`println!`), parameter and return-type annotations, a `range` builtin, an
  entry function, and value rendering with each language's literal words.
- **Stream kernel**: reads the same definitions by its own method. One
  language module, `kernels/stream/src/language/`, registers handlers for
  the constructs a definition spells and transforms the token stream by its
  block style, comment markers, prologue, variable prefix and case rules;
  the Python-like and Rust-like modules are removed. Both kernels read one
  identifier character class.
- **Host**: `--lang` (or `--language`) takes an embedded language name
  (`python`), its extension (`py`), or the path of a definition file; the
  file extension decides otherwise, and Lumen is the default. Definitions
  live in `langs/`.
- **Languages**: the Python and Rust examples are written as Python and
  Rust, and PHP, Ruby, Pascal, C, JavaScript and Swift examples are
  added; every example runs on both kernels, and `scripts/kernel_diff.sh`
  compares their output
  program by program.
- **Removed**: `yaml/`, `grammar/`, the BNF document and the language
  comparison document, all superseded by the definitions and their table.

---

## v0.2.0 - 2026-09-06
**Contributors:** Ivan Shim orchestrating, Claude Fable 5.1 coding
**Release:** Cleanup and kernel-fidelity pass; both kernels now match their charters

### What was done:
- **Workspace**: one host binary (`src/main.rs`) and two kernel library crates
  (`kernels/stream`, `kernels/microcode`) that cannot import each other. No
  more spawning sibling binaries; `lumen-lang --kernel stream|microcode`.
- **Stream kernel purity**: the lexer no longer strips comments or assumes
  which bytes form identifiers (languages supply both); the environment lost
  its memoization policy, its Lumen-typed array mutation and its unsafe scope
  guard, gaining `update`, `get_mut`, `with_scope` and a typed extension slot.
  Lumen's memoization moved into the language layer.
- **Microcode kernel as data**: languages are YAML schemas
  (`kernels/microcode/schemas/`); the four stages read tables and contain no
  language names. Seven primitives plus one internal loop; `for`, `until`,
  functions, indexed assignment and the pipe operator are desugared. One exact
  numeric tower replaces per-type arithmetic and fixes real-number comparison.
- **Dialect fixes**: the Rust-like and Python-like examples had been failing
  on both kernels (infinite loops on the stream kernel from assignment
  shadowing inside loop scopes; unknown `print` on the microcode kernel).
  All 186 example runs pass.
- **Independence**: `scripts/kernel_independence.py` in CI rejects any
  cross-crate reference and any run of five or more identical source lines
  between the kernels; the two kernels take deliberately different routes to
  comment removal and binding storage.
- **Hygiene**: zero compiler warnings and CI that denies them; GitHub Actions
  runs every example on both kernels; stale files, dead helpers and stale
  docs removed; README and kernel documents rewritten to match the code.

---

## v0.0.7 - 2026-01-07
**Contributors:** Ivan Shim orchestrating, GPT-5.2 consulting, Claude Code Haiku 4.5 coding (Happy Birthday Ivan!)
**Release:** Microcode kernel rewritten and optimized (7 primitives retained), codebase cleanup and standardization

### What was done:

- **Microcode Kernel Rewrite and Optimization**:
  - Complete architectural redesign of `src_microcode/kernel/` with improved efficiency and clarity
  - Retained canonical 7 primitives: Sequence, Scope, Branch, Assign, Invoke, Operate, Return
  - Optimized 4-stage pipeline: Ingest → Structure → Reduce → Execute
  - Enhanced schema-driven execution model while maintaining language-agnostic design
  - All 68 tests passing with identical results across both kernels

- **Directory Structure Standardization**:
  - Removed "mini-" prefixed YAML language specifications: `python.yaml`, `rust.yaml`
  - Removed "mini-" prefixed EBNF grammar files: `python.ebnf`, `rust.ebnf`
  - Renamed example directories: `examples/python/` → `examples/python/`, `examples/rust/` → `examples/rust/`
  - Result: Cleaner, more intuitive naming convention across all project directories

- **Test Suite Updates**:
  - Updated `test_all.sh` to reference new example directory paths (`examples/python/`, `examples/rust/`)
  - Updated language identifiers in test logic from `"python"` to `"python"` and `"rust"` to `"rust"`
  - Updated output labels to match new naming scheme
  - Test suite remains fully functional with all 68 tests passing

- **Documentation Cleanup**:
  - Removed `apply_word_boundary_changes.md` (process documentation for word-boundary keyword implementation)
  - Removed `claude_unblock_prompt.md` (technical prompt for debugging keyword-in-identifier issues)
  - Removed `COMPARISON_AI_VS_LUMEN.md` (detailed comparison between ai.yaml ML design and lumen.yaml general-purpose design)
  - Result: Reduced documentation clutter, retained only maintained design documentation

### Key Achievements:
- ✅ Microcode kernel completely rewritten with enhanced architecture and optimization
- ✅ 7-primitive execution model preserved and refined (Sequence, Scope, Branch, Assign, Invoke, Operate, Return)
- ✅ Unified naming convention across yaml/, grammar/, and examples/ directories
- ✅ All test infrastructure updated and fully operational (68 tests passing)
- ✅ Temporary development documentation removed (678 lines of clutter eliminated)
- ✅ Cleaner project structure with improved clarity and maintainability
- ✅ Zero regressions: all tests passing on both Stream and Microcode kernels

---

## v0.0.6 - 2026-01-04
**Contributors:** Ivan Shim orchestrating, GPT-5.2 consulting, Claude Code Haiku 4.5 coding
**Release:** Dual-kernel architecture: Stream and Microcode kernels

### What was done:

- **Dual-Kernel Refactor**:
  - Original kernel refactored into `src_stream/` (procedural, AST-based)
  - New `src_microcode/` kernel created (data-driven, schema-based)
  - Both kernels execute identical language specifications independently
  - Zero code sharing between kernels to explore separate execution philosophies

- **Stream Kernel (`src_stream/`)**: Traditional Interpreter Architecture
  - Tree-walking AST evaluator: Parse → AST → Evaluate
  - Language-agnostic kernel with trait-based handler dispatch
  - Complete implementations of Lumen, Rust, and Python
  - Procedural language definitions with explicit parsing and evaluation logic
  - All 35 example programs execute correctly on Stream kernel

- **Microcode Kernel (`src_microcode/`)**: Data-Driven Schema Architecture
  - 4-stage execution pipeline: Ingest → Structure → Reduce → Execute
  - Declarative schema system: All language semantics defined via tables/schemas
  - Kernel contains zero language-specific code (fully data-driven)
  - Language schemas specify tokens, operators, precedence, and rules
  - Complete schema implementations for Lumen, Rust, and Python
  - All 35 example programs execute correctly on Microcode kernel

- **Multi-Language Support in Both Kernels**:
  - Lumen: Python-style indentation (24 examples)
  - Rust: Rust-style curly braces (5 examples)
  - Python: Python-like syntax (5 examples)
  - Each language runs identically on both kernels (68 total tests, all passing)

- **Architecture Achievement**:
  - Demonstrated complete separation of kernel mechanics from language semantics
  - Each kernel explores a different design philosophy while supporting identical features
  - Verified both approaches produce identical execution results across all test cases
  - Foundation for exploring multiple execution strategies without code duplication

### Key Achievements:
- ✅ Two completely independent kernel implementations coexist
- ✅ Three languages fully supported on both kernels (100% test pass rate)
- ✅ Schema-driven design proves language semantics can be purely declarative
- ✅ AST-based design proves traditional tree-walking works equivalently
- ✅ Zero shared code between kernels enables architectural exploration
- ✅ Comprehensive test suite validates dual-kernel equivalence (68 tests)

---

## v0.0.5 - 2026-01-03
**Contributors:** Ivan Shim orchestrating, GPT-5.2 prompting, Claude Code Haiku 4.5 coding
**Release:** Kernel ontological neutrality, extern correctness, and mathematical proof programs

### What was done:

- **Kernel Refactor to Ontologically Neutral Value System**:
  - Replaced concrete `Value` enum with opaque `RuntimeValue` trait in kernel
  - Kernel now treats all values as abstract types, makes no semantic assumptions
  - Created language-specific value types: `LumenNumber`, `LumenBool`, `LumenString` (Lumen), `MiniRustNumber`, `MiniRustBool` (Rust), `MiniPythonNumber`, `MiniPythonBool` (Python)
  - Updated all expressions and statements across all three languages to use language-specific constructors and helpers
  - Implemented safe type downcasting via `as_any()` trait method and language-specific helper functions (`as_number()`, `as_bool()`, `as_string()`)
  - Result: Kernel is now language-independent; all value semantics belong to language modules

- **Extern System Correctness Enforcement**:
  - Fixed design drift: Parser now requires extern selectors to be **string literals**, not identifiers
  - Reject unquoted identifiers (e.g., `extern(print_native, ...)`) with clear error messages
  - Updated all 9 extern example files to use quoted selectors (e.g., `extern("print_native", ...)`)
  - Result: Selectors are now opaque data strings; Lumen makes no assumptions about capability names

- **π and e Examples: Integer-Only, Fixed-Point Implementations**:
  - Replaced all π and e examples with mathematically correct, deterministic integer-only implementations
  - **e (Euler's number)**: Factorial series implementation: e = Σ(1/n!) scaled by SCALE = 10^10
  - **π (Pi)**: Machin's formula with arctangent series: π = 16·arctan(1/5) - 4·arctan(1/239)
  - All arithmetic uses integer operations; decimal point inserted only at output time
  - Updated 6 example files across 3 languages (Lumen, Python, Rust)
  - Output format: Separated integer and fractional parts using modulo and division
  - Result: Canonical proof programs demonstrating deterministic integer math for each language

### Key Achievements:
- ✅ Kernel contains zero language-specific assumptions
- ✅ Strings properly implemented as language-level values (not kernel assumptions)
- ✅ Extern system shaped for host-agnostic extensibility
- ✅ Proper abstraction ordering: Kernel → Strings → Extern
- ✅ Clear separation of concerns: Kernel owns mechanics, languages own semantics
- ✅ Canonical proof programs for language correctness

---

## v0.0.4 - 2026-01-02
**Contributors:** Ivan Shim, GPT-5.2 prompting & Claude Code Haiku 4.5 coding
**Release:** Language consolidation and PythonCore addition

### What was done:
- **Lexical Scoping Implementation**: Added block-scoped environments with proper variable resolution:
  - Each `if/else` block and loop iteration creates a new scope
  - Variable assignments search parent scopes lexically
  - Inner scope variables don't leak to outer scopes
  - All 7 language implementations updated with scoping support
  - 6 new scope test programs demonstrate correct behavior
- **Language Consolidation**: Archived 5 inactive language implementations:
  - `src_mini_php/` → `archive/src_mini_php/`
  - `src_mini_sh/` → `archive/src_mini_sh/`
  - `src_mini_c/` → `archive/src_mini_c/`
  - `src_mini_apple_pascal/` → `archive/src_mini_apple_pascal/`
  - `src_mini_apple_basic/` → `archive/src_mini_apple_basic/`
- **PythonCore Implementation**: New language module with full feature parity:
  - Indentation-based blocks (Python-like syntax)
  - All expression types: literals, variables, arithmetic, comparison, logical
  - All statement types: assignment, if/else, while, print, break, continue
  - 5 example programs: loop, fibonacci, demo, pi (1000 iterations), e (10 terms)
  - File extensions: `.py`
- **Project Cleanup**: Updated `src/main.rs` to support only 3 active languages:
  - Lumen (`.lm`)
  - RustCore (`.rs`)
  - PythonCore (`.py`)
- **Test Suite Update**: Modified `test_all.sh` for 3-language support (21 total tests)
- **Build Status**: All tests passing, zero compilation errors

---

## v0.0.3 - 2026-01-01
**Contributors:** Ivan Shim & Claude Code Haiku 4.5
**Release:** Lumen multi-language kernel: lumen, rust, php, sh, c, apple pascal, apple basic

### What was done:
- Renamed `framework` module to `kernel` (language-agnostic kernel)
- Implemented 6 additional language modules with full feature parity:
  - **Rust**: C-style operators (`&&`, `||`, `!`), `let` keyword, semicolons
  - **Mini-PHP**: PHP-style variables (`$var`), `echo` output
  - **Mini-Shell**: Shell-style variables in expressions, shell-like syntax
  - **Mini-C**: C-style syntax, `printf` output
  - **Mini-Pascal**: Pascal-style `:=` assignments, `BEGIN...END` blocks
  - **Mini-Basic**: BASIC-style `LET` and `PRINT` keywords (uppercase)
- Implemented dual language selection system:
  - Priority 1: Explicit `--lang` parameter
  - Priority 2: File extension detection (`.rs`, `.php`, `.sh`, `.c`, `.p`, `.basic`)
- Renamed `demo_v0_1` examples to `demo` across all language modules
- Created mathematical computation examples (pi and e) for all 7 languages
- Updated loop counts: fibonacci (20 → 10), loop (5 → 10)
- Built comprehensive test suite (`test_all.sh`) with auto-discovery
- Fixed EOF token handling for all mini-language modules
- All 35 example programs passing tests

---

## v0.0.2 - 2025-12-31
**Contributors:** Ivan Shim & Claude Code Haiku 4.5
**Release:** Language-agnostic framework architecture

### What was done:
- **Framework/Language Separation**: Split monolithic codebase into language-agnostic `src/framework/` and language-specific `src_lumen/` modules
- **Removed Structural Concepts from Framework**: Eliminated all hardcoded token logic (NEWLINE, INDENT, DEDENT, EOF) from framework parser, lexer, and registry
- **Language-Specific Structural Parsing**: Moved all indentation, newline, and block parsing logic to `src_lumen/structure/structural.rs`
- **Generic Parser**: Framework parser now purely generic—delegates all parsing decisions to registered handlers via trait-based dispatch
- **Plugin Architecture**: Languages can now define custom syntax, tokens, and operators by implementing and registering handlers
- **Documentation Consolidation**: Reorganized docs; BNF.md is now the authoritative grammar specification
- **Verified Functionality**: All examples (loop.lm, fibonacci.lm, demo.lm) tested and working
- **Architectural Achievement**: Framework is now completely language-agnostic. Can support multiple languages with completely different syntax and semantics using the same framework core.

---

## v0.0.1 - 2025-12-30
**Contributors:** Ivan Shim & GPT-5.2
**Release:** Initial working interpreter

### What was done:
- Implemented a full parse → AST → evaluate execution pipeline
- Added indentation-based block parsing
- Implemented `while` loops and `if/else` conditionals
- Added variables, arithmetic, comparisons, and `print()`
- Delivered the first complete, executable Lumen program

---

