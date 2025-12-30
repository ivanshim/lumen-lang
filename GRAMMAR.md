Lumen Grammar Overview (v0.1)

Lumen uses a Python-style indentation-based grammar.
Blocks are introduced by ':' and defined by increased indentation.

Statements are line-oriented.
Each statement occupies exactly one line unless it introduces a block.

There are no semicolons.
There are no user-defined functions.
There is exactly one builtin function: print().

Control flow is explicit and minimal.
There is no implicit truthiness: comparisons yield booleans.

Indentation rules:
- Exactly 4 spaces per indentation level
- Tabs are not supported
- Mixed indentation is an error

Evaluation model:
- Programs are executed top-to-bottom
- while and if introduce lexical blocks
- break and continue apply only to the nearest enclosing while

This grammar is intentionally small and stable.
Future versions may extend the grammar but will not change v0.1 semantics.
