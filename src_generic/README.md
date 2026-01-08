# Stream Kernel 2: Quick Start Guide

A fresh implementation of the stream kernel using opaque analysis architecture.

## What is This?

A **language-agnostic structure processing kernel** that:
- Takes token streams as input
- Inserts block markers (indentation → braces) as needed
- Works identically for three languages: Lumen, Python, Rust
- Uses **zero semantic knowledge** in the kernel itself

## Build & Run

```bash
# Build the binary
cargo build --bin stream2

# Run the demonstration
cargo run --bin stream2

# Run the test suite
cargo test --bin stream2
```

## Key Concepts

### Kernel
- **One trait**: `StructureProcessor`
- **One method**: `process_structure(tokens) → tokens`
- **Size**: ~70 lines
- **Knowledge**: Zero (no language concepts)

### Languages
Each language implements the `StructureProcessor` trait with two methods:
- `analyze(&token)` → Opaque semantic data
- `handle_change(prev, curr)` → Tokens to insert

### Flow

```
Language provides:
  - Token stream
  - StructureProcessor implementation

Kernel does:
  for each token:
    analyze it (ask language)
    handle changes (ask language)
    insert results
    yield token

Output:
  - Expanded token stream with markers
```

## File Tree

```
src_stream_2/
├── kernel/mod.rs                    ← Pure kernel (read this first!)
├── languages/
│   ├── lumen/structure.rs           ← Indent-based (4 spaces = 1 level)
│   ├── python_core/structure.rs     ← Indent + colon handling
│   └── rust_core/structure.rs       ← Explicit braces (passthrough)
├── main.rs                          ← Examples and demonstration
├── ARCHITECTURE.md                  ← Deep dive
└── README.md                        ← This file
```

## Examples

### Lumen (Indentation-based)

Input:
```
if x
    print(x)
```

Tokens: `if`, `x`, `newline`, `indent(4sp)`, `print`, `(`...

Output:
```
if
x
newline
marker_block_start      ← INSERTED
indent
marker_block_end        ← INSERTED (at dedent)
print
(
...
```

### Python (Indentation + Colon)

Input:
```
if x:
    print(x)
```

Output:
```
if
x
colon
newline
marker_block_start      ← INSERTED (colon seen)
indent
marker_block_end        ← INSERTED
print
...
```

### Rust (Explicit Braces)

Input:
```
if x {
    print(x)
}
```

Output:
```
if
x
{
print
(
...
}
```

(No insertions - braces already explicit)

## Tests

8 unit tests, all passing:

```
✓ kernel::tests::test_kernel_processes_tokens
✓ kernel::tests::test_kernel_inserts_tokens
✓ lumen::structure::tests::test_lumen_analyzes_indent
✓ lumen::structure::tests::test_lumen_handles_depth_increase
✓ lumen::structure::tests::test_lumen_handles_depth_decrease
✓ python_core::structure::tests::test_python_analyzes_indent
✓ python_core::structure::tests::test_python_detects_colon
✓ rust_core::structure::tests::test_rust_passes_through
```

## Adding a New Language

1. Create `src_stream_2/languages/mylan/structure.rs`
2. Define your semantic data: `pub struct MyLanAnalysis { ... }`
3. Implement `StructureProcessor`:
   ```rust
   impl StructureProcessor for MyLanProcessor {
       fn analyze(&self, token: &Token) -> OpaqueAnalysis {
           // Extract what matters for your language
           Box::new(MyLanAnalysis { ... })
       }

       fn handle_change(&self, prev, curr) -> Vec<Token> {
           // Decide what tokens to insert
           vec![...]
       }
   }
   ```
4. Create `src_stream_2/languages/mylan/mod.rs`
5. Export: `pub use structure::my_lan_structure;`
6. Update `src_stream_2/languages/mod.rs`

The kernel doesn't change. You've added a language.

## Design Philosophy

**Kernel Rule**: "I don't know what language constructs mean. I only know how to process tokens systematically."

**Language Rule**: "I interpret my language's tokens and tell the kernel what to do. The kernel doesn't need to understand me."

This achieves:
- ✅ Semantic purity (kernel is dumb)
- ✅ Language independence (kernel is generic)
- ✅ Code reuse (loop written once)
- ✅ Easy extension (one trait per language)

## What's NOT Here

This is structure processing only. A full stream kernel would also need:
- Lexer (tokenization)
- Parser (statement/expression parsing)
- Evaluator (execution)
- Built-ins (print, len, etc.)

See `src_stream/` for a more complete implementation (though less clean architecturally).

## Further Reading

- `ARCHITECTURE.md` - Deep technical details
- `src_stream_2/kernel/mod.rs` - Kernel implementation (read the code!)
- `src_stream_2/languages/*/structure.rs` - Language implementations

## Questions?

The best way to understand this is to:
1. Read `kernel/mod.rs` (it's short!)
2. Read one language implementation (e.g., `lumen/structure.rs`)
3. Run `cargo run --bin stream2` to see the output
4. Modify one of the examples and rerun
