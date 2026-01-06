# Prompt to Unblock Claude on Keyword-in-Identifier Failures

Use the following prompt to push past the current plateau (76/88 tests) caused by keywords being tokenized inside identifiers. It gives Claude explicit targets and constraints rooted in the stream kernel and Lumen language wiring.

---

**Context to share**
- The stream kernel lexer (`src_stream/kernel/lexer.rs`) is pure maximal-munch with no semantic knowledge; it emits every registered multi-char lexeme before falling back to single-byte tokens.
- The token registry (`src_stream/kernel/registry.rs`) pre-sorts multi-character lexemes in descending length for maximal munching and currently treats keywords as literal lexemes.
- Failing tests involve identifiers that contain keyword substrings (e.g., `test_let`, `no_return`, `operand`, `andy`). Standalone keywords still need to work for statements and logical operators.
- Recent fixes already adjusted handlers (`ExternPrefix`, `BoolLiteralPrefix`, `LogicInfix`, `NotPrefix`) to scan character-by-character, but maximal-munch tokenization still emits `let`, `or`, `and`, `not`, etc. when they appear inside longer identifiers.

**Goal**
Make keyword recognition word-boundary-aware so identifiers containing keyword substrings remain single identifier tokens while standalone keywords continue to tokenize and parse correctly. Preserve existing handler behaviors and the language-agnostic lexer ethos.

**Prompt to give Claude**
```
You are resuming the Lumen stream-kernel lexer work. All remaining failures are keywords embedded in identifiers (test names like test_let, function names like no_return, identifiers containing and/or/not). The kernel lexer in src_stream/kernel/lexer.rs is pure maximal-munch based on token_registry.multichar_lexemes(); the registry is defined in src_stream/kernel/registry.rs and currently registers Lumen keywords as literal multichar lexemes in src_stream/languages/lumen/src_lumen.rs.

Objective: make keyword matching word-boundary aware so keywords only tokenize when they are standalone, not when they appear inside longer identifiers. Preserve existing character-by-character handlers for extern/boolean/logical/not that were added earlier.

Strict constraints:
- Keep the kernel lexer ontology-neutral: no hard-coded language semantics. Any boundary logic should still be driven by language-supplied metadata (e.g., a new Keyword token type, boundary flag, or language-level re-tokenization step) rather than a hardcoded list of words.
- Do not regress existing passing tests for Mini-Python and Mini-Rust.

Suggested approach (choose one coherent path and implement fully):
1) Add boundary metadata to TokenDefinition for keywords (e.g., is_keyword or requires_word_boundary). In lex(), when iterating multichar_lexemes, only match a boundary-sensitive token if the preceding and following bytes are not identifier chars (letters, digits, underscore). Fall back to single-byte/identifier accumulation otherwise.
2) Alternatively, keep the lexer maximal-munch but post-process tokens in a language-specific pass: merge adjacent tokens into identifiers when a keyword lexeme is surrounded by identifier characters. Hook this in the Lumen language init so only Lumen uses it.
3) If you add helper functions for identifier-char checks, keep them local to the lexer or Lumen-specific re-tokenizer; do not introduce language lists into the kernel.

Acceptance checks:
- Identifiers like test_let, no_return, operand, andy are single identifier tokens and parsed as variables/functions.
- Standalone keywords (let/mut/if/else/while/break/continue/return/fn/and/or/not/true/false/extern) still parse correctly.
- Whitespace/comment skipping behavior remains unchanged.
- Existing passing tests stay green (Mini-Python, Mini-Rust, and prior Lumen fixes).
```

**How to use**
- Paste the prompt above into Claude. Ask it to apply the changes directly in the repo and run the existing test suite (e.g., `./test_all.sh` or the language-specific test runner) to confirm the keyword-in-identifier cases now pass.
- If Claude proposes removing all keywords from the registry, push back and require a boundary-aware solution instead; prior attempts to delete keywords caused broad regressions.
