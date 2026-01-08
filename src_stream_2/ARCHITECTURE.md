# Stream Kernel 2: Opaque Analysis Architecture

## Overview

`src_stream_2` is a complete reimplementation of the stream kernel using the **opaque analysis** design pattern, achieving:

- ✅ **Zero semantic knowledge in kernel** - Kernel never mentions language concepts
- ✅ **100% language-agnostic processing** - Same kernel code for all languages
- ✅ **Pure separation of concerns** - Kernel does orchestration, languages do interpretation
- ✅ **Extensible design** - Add new languages by implementing one trait

## Architecture

### Kernel: Dumb Orchestrator

**File**: `src_stream_2/kernel/mod.rs`

The kernel contains **no language knowledge**. It only:
1. Iterates through tokens
2. Asks the language to analyze each token
3. Asks the language what to do when analysis changes
4. Mechanically applies the results

```rust
pub trait StructureProcessor: Send + Sync {
    fn analyze(&self, token: &Token) -> OpaqueAnalysis;
    fn handle_change(
        &self,
        prev_analysis: Option<&OpaqueAnalysis>,
        curr_analysis: &OpaqueAnalysis,
    ) -> Vec<Token>;
}
```

The kernel:
- Never knows what `OpaqueAnalysis` contains
- Never interprets the data
- Just passes it around and asks for decisions
- **~70 lines of code, zero semantic knowledge**

### Languages: Smart Interpreters

Each language implements `StructureProcessor` and provides:

1. **Semantic data structure** - What the language extracts from tokens
2. **Analyze function** - Extract properties from a token
3. **Handle_change function** - Decide what tokens to insert based on property changes

#### Example: Lumen

**File**: `src_stream_2/languages/lumen/structure.rs`

```rust
// Lumen's semantic data (kernel never sees this type name)
pub struct LumenAnalysis {
    pub depth: i32,
    pub is_newline: bool,
}

impl StructureProcessor for LumenStructureProcessor {
    fn analyze(&self, token: &Token) -> OpaqueAnalysis {
        // Extract properties: 4 spaces = 1 depth level
        let depth = token.lexeme.len() / 4;
        Box::new(LumenAnalysis { depth: depth as i32, ... })
    }

    fn handle_change(&self, prev: Option<&OpaqueAnalysis>, curr: &OpaqueAnalysis) {
        // Downcast to Lumen's type (kernel doesn't)
        let curr = curr.downcast_ref::<LumenAnalysis>().unwrap();

        // Interpret: depth increase = block start
        if curr.depth > prev.depth {
            vec![Token::new("marker_block_start", "{")]
        }
    }
}
```

#### Example: Python

**File**: `src_stream_2/languages/python_core/structure.rs`

Similar structure, but with colon handling and Python semantics.

#### Example: Rust

**File**: `src_stream_2/languages/rust_core/structure.rs`

Returns empty insertions because Rust's braces are explicit in the token stream.

## Key Design Decisions

### 1. Opaque Analysis Type

```rust
pub type OpaqueAnalysis = Box<dyn Any + Send + Sync>;
```

- Kernel receives and passes around this type
- Language owns what's inside
- No shared data structures between kernel and languages
- **Achieves true semantic blindness**

### 2. No Trait Objects for Semantics

The language doesn't expose its semantic data types to the kernel. Only through downcasting in handle_change does it reveal the concrete type.

```rust
// Kernel never sees LumenAnalysis
// Language downcasts inside handle_change
let analysis = curr_analysis.downcast_ref::<LumenAnalysis>().unwrap();
```

### 3. Simple Processing Loop

Kernel's job is trivial:

```rust
for token in tokens {
    let curr = self.processor.analyze(&token);
    let inserted = self.processor.handle_change(prev.as_ref(), &curr);
    result.extend(inserted);
    result.push(token);
    prev = Some(curr);
}
```

This pattern is **reusable** across all languages while remaining completely generic.

## File Structure

```
src_stream_2/
├── kernel/
│   └── mod.rs                    # Pure kernel (70 lines, zero semantics)
│
├── languages/
│   ├── mod.rs
│   ├── lumen/
│   │   └── structure.rs          # Lumen: indentation-based blocks
│   ├── rust_core/
│   │   └── structure.rs          # Rust: explicit braces (passthrough)
│   └── python_core/
│       └── structure.rs          # Python: indentation + colon
│
├── main.rs                       # Demonstration and examples
└── ARCHITECTURE.md               # This file
```

## Adding a New Language

To add a new language (e.g., `javascript`):

1. Create `src_stream_2/languages/javascript/structure.rs`
2. Define your semantic data structure
3. Implement `StructureProcessor` trait
4. Export through `mod.rs`
5. Create a kernel instance and use it

That's it. The kernel doesn't change.

## Test Coverage

All components have unit tests:

- **Kernel tests** (8 tests total):
  - `test_kernel_processes_tokens` - Basic token processing
  - `test_kernel_inserts_tokens` - Token insertion capability

- **Lumen tests**:
  - `test_lumen_analyzes_indent` - Indent detection
  - `test_lumen_handles_depth_increase` - Block start markers
  - `test_lumen_handles_depth_decrease` - Block end markers

- **Python tests**:
  - `test_python_analyzes_indent` - Multi-level indentation
  - `test_python_detects_colon` - Colon handling

- **Rust tests**:
  - `test_rust_passes_through` - No transformation

**Run tests:**
```bash
cargo test --bin stream2
```

## Example Behavior

### Lumen Input
```
if x
    print(x)
```

Tokens: `if`, `x`, `newline`, `indent(4 spaces)`, `print`, `(`, `x`, `)`

Processing:
1. Token: `if` → analyze → depth=0, insert nothing
2. Token: `x` → analyze → depth=0, insert nothing
3. Token: `newline` → analyze → depth=0, insert nothing
4. Token: `indent(4 spaces)` → analyze → **depth=1**, insert block start
5. Continue...

Output tokens include `marker_block_start` and `marker_block_end` at appropriate positions.

## Design Benefits

| Aspect | Benefit |
|--------|---------|
| **Kernel purity** | Zero language knowledge, can be tested independently |
| **Language simplicity** | Only implement interpret semantics, not processing algorithms |
| **Extensibility** | New languages require minimal code (~50 lines) |
| **Maintainability** | Changes to language don't affect kernel, vice versa |
| **Code reuse** | Token processing loop is written once, used by all |
| **Testing** | Kernel behavior can be verified with mock processors |

## Limitations & Future Work

Current implementation:
- Structure processing only (block markers)
- Not integrated with full lexer/parser/evaluator from src_stream
- Demonstration of architectural pattern only

To build a full stream kernel:
1. Integrate with src_stream's lexer (or implement new one)
2. Add statement/expression parsing with similar opaque pattern
3. Add evaluation/execution phase
4. Integrate operators and built-in functions

## Comparison to Previous Approaches

| Aspect | Option 3 (Complete Abstraction) | Opaque Analysis (Current) |
|--------|--------------------------------|--------------------------|
| Kernel code | ~5 lines | ~70 lines |
| Kernel responsibility | None (pure delegation) | Orchestration algorithm |
| Language burden | Full (must implement everything) | Minimal (only semantics) |
| Code reuse | None (each language reinvents) | High (all use same loop) |
| Purity | Absolute | Algorithmic + semantic |
| Testability | Hard (kernel is trivial) | Easy (testable patterns) |

Current approach is superior for practical use while maintaining semantic purity.

## Conclusion

This architecture demonstrates that a kernel can be **completely ignorant of language semantics** while still providing value through reusable algorithmic patterns. The key insight is distinguishing between:

- **Semantic knowledge** (what things mean) → Language owns this
- **Algorithmic patterns** (how to process systematically) → Kernel owns this

This separation enables true language-agnosticism without sacrificing code organization or extensibility.
