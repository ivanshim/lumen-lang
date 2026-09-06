# Language definitions

One JSON file per language. Each file maps a fixed set of labels, the names
of things the kernel can do, to the strings a language spells them with. The
semantics behind every label belong to the kernel and are the same for every
language: a definition changes how a program is spelled, never what it means.

Both kernels read these files. The microcode kernel reads every label of
all four, embedded at build time and picked by file extension or `--lang`;
`lumen-lang --config file.json` reads a definition from disk instead. The
stream kernel hosts Lumen only and takes every spelling its handlers match
from `lumen.json`, so an alias added there reaches both kernels at once.

These files replace the earlier YAML specifications, EBNF grammars and
grammar documents: a language's surface syntax is its definition, its
semantics are the kernel's, and the table below is the comparison.

## Format rules

1. A file is one flat JSON object. Every file carries the same labels in the
   same order; `scripts/config_table.py` fails if they differ, and the
   kernel's reader rejects a missing label, an unknown key, or a value of
   the wrong type.
2. Lexeme labels take a list of strings. The first entry is canonical and is
   what the kernel uses when it renders a value or names a token in an error.
   Later entries are aliases. An empty list means the language has no
   spelling for that label.
3. Setting labels take a JSON scalar of the setting's type: `true`/`false`,
   a number, or a string. `null` means the setting does not apply.
4. `op.precedence` is a list of tiers, lowest first, each tier a list of
   lexemes. Precedence is keyed by lexeme, not by label, so two aliases of one
   operation may sit in different tiers (PHP's `and` and `&&`). A lexeme that
   is both binary and unary (`-`) takes its binary precedence from the first
   tier it appears in and its unary precedence from the last.
   `op.right_associative` names the lexemes that associate to the right.
5. Keys beginning with `$comment` are ignored.
6. A lexeme may appear under at most one label per parsing position. `-`
   under both `op.sub` (infix) and `op.negate` (prefix) is allowed; the same
   string under two infix labels is an error.
7. Every lexeme under `stmt.*` and `literal.*`, and every word-shaped
   operator (`and`, `not`), is a reserved word and must be shaped like an
   identifier.
8. `format_version` identifies the label set and must be `1`.
9. Identifier characters are a kernel policy selected by `identifier.unicode`:
   underscore always, then letters and digits from ASCII or from all of
   Unicode. `identifier.variable_prefix` names a character (`$`) that begins a
   variable and is part of its name. Keywords and operators are always ASCII.
10. `lexical.keywords_case_insensitive` folds any word that spells a
    reserved word to lower case; `identifier.case_insensitive` folds every
    other word. Both default to false.
11. Strings: every quote in `lexical.string_quotes` delimits a string.
    Inside a quote listed in `lexical.raw_quotes` only `\\` and the
    backslash-escaped quote itself are escapes; inside any other quote the
    letters in `lexical.string_escapes` are escapes too (`n`, `t`, `r`, `0`,
    a backslash, or a quote character). A backslash before an unlisted
    letter is kept as written.
12. A builtin name is an identifier that may end in one operator character
    of the language, which is how Rust's `println!` is a name.
13. `print` and `write` join their arguments with single spaces. When the
    first argument is a string holding `{}` placeholders and more arguments
    follow, the placeholders are filled in order instead.
14. `system.entry` names a function (Rust's `main`) that the kernel calls
    after the program body if the body defined it.
15. Statement forms are fixed by the kernel: `stmt.let` introduces a binding
    with an optional `stmt.let.mutable` word and an optional
    `stmt.let.annotation` token followed by one type word; `stmt.if` takes a
    condition and a block, then any number of `stmt.elif` branches and one
    `stmt.else`; `stmt.while` and `stmt.until` take a condition and a block,
    the latter testing after each pass; `stmt.for` binds a variable over a
    range with `stmt.for.in`; `stmt.function` takes a name, a parenthesised
    parameter list whose entries may carry the binding annotation, an
    optional `stmt.function.returns` token with one type word, and a block.

## Labels without a kernel path yet

These describe real syntax the target languages have, but the kernel does
not implement them. A definition must leave them empty; the reader rejects
a non-empty value and names the label. Each is a kernel change, not a
definition change:

- `syntax.map.*`: no map value in the kernel, so Python dictionary literals
  and PHP associative arrays are left out.
- `stmt.foreach.*`, and `stmt.for` over a collection rather than a range: no
  iteration over collections.
- `stmt.emit`: output as a statement rather than a call, PHP's `echo`.

## Comparison

Generated by `python3 scripts/config_table.py`; edit the JSON, not the table.

<!-- table:start -->
| Label | lumen | python | php | rust |
|---|---|---|---|---|
| `format_version` | `1` | `1` | `1` | `1` |
| `language` | `lumen` | `python` | `php` | `rust` |
| `extensions` | `lm` | `py` | `php` | `rs` |
| `lexical.comment_line` | `#` | `#` | `//` `#` | `//` |
| `lexical.comment_block.open` | - | - | `/*` | `/*` |
| `lexical.comment_block.close` | - | - | `*/` | `*/` |
| `lexical.string_quotes` | `"` `'` | `"` `'` | `"` `'` | `"` |
| `lexical.raw_quotes` | `'` | - | `'` | - |
| `lexical.string_escapes` | `n` `t` `\` `"` | `n` `t` `\` `"` `'` | `n` `t` `\` `"` | `n` `t` `\` `"` |
| `lexical.prologue` | - | - | `<?php` | - |
| `lexical.number.decimal_point` | `.` | `.` | `.` | `.` |
| `lexical.number.base_marker` | `@` | - | - | - |
| `lexical.number.exponent_marker` | `^` | - | - | - |
| `lexical.number.hex_prefix` | - | `0x` | `0x` | `0x` |
| `lexical.keywords_case_insensitive` | `false` | `false` | `true` | `false` |
| `identifier.unicode` | `true` | `true` | `true` | `true` |
| `identifier.variable_prefix` | - | - | `$` | - |
| `identifier.case_insensitive` | `false` | `false` | `false` | `false` |
| `block.style` | `indentation` | `indentation` | `braces` | `braces` |
| `block.open` | - | - | `{` | `{` |
| `block.close` | - | - | `}` | `}` |
| `block.intro` | - | `:` | - | - |
| `block.indent_size` | `4` | `4` | - | - |
| `stmt.terminator` | `;` | `;` | `;` | `;` |
| `syntax.group.open` | `(` | `(` | `(` | `(` |
| `syntax.group.close` | `)` | `)` | `)` | `)` |
| `syntax.call.open` | `(` | `(` | `(` | `(` |
| `syntax.call.separator` | `,` | `,` | `,` | `,` |
| `syntax.call.close` | `)` | `)` | `)` | `)` |
| `syntax.array.open` | `[` | `[` | `[` | `[` |
| `syntax.array.separator` | `,` | `,` | `,` | `,` |
| `syntax.array.close` | `]` | `]` | `]` | `]` |
| `syntax.map.open` | - | - | - | - |
| `syntax.map.separator` | - | - | - | - |
| `syntax.map.pair` | - | - | - | - |
| `syntax.map.close` | - | - | - | - |
| `literal.true` | `true` | `True` | `true` | `true` |
| `literal.false` | `false` | `False` | `false` | `false` |
| `literal.null` | `null` | `None` | `null` | `None` |
| `op.right_associative` | `**` | `**` | `**` | - |
| `op.add` | `+` | `+` | `+` | `+` |
| `op.sub` | `-` | `-` | `-` | `-` |
| `op.mul` | `*` | `*` | `*` | `*` |
| `op.div` | `/` | `/` | `/` | - |
| `op.quot` | `//` | `//` | - | `/` |
| `op.rem` | `%` | `%` | `%` | `%` |
| `op.pow` | `**` | `**` | `**` | - |
| `op.eq` | `==` | `==` | `==` | `==` |
| `op.ne` | `!=` | `!=` | `!=` | `!=` |
| `op.lt` | `<` | `<` | `<` | `<` |
| `op.le` | `<=` | `<=` | `<=` | `<=` |
| `op.gt` | `>` | `>` | `>` | `>` |
| `op.ge` | `>=` | `>=` | `>=` | `>=` |
| `op.and` | `and` | `and` | `&&` `and` | `&&` |
| `op.or` | `or` | `or` | `\|\|` `or` | `\|\|` |
| `op.not` | `not` `!` | `not` | `!` | `!` |
| `op.negate` | `-` | `-` | `-` | `-` |
| `op.concat` | `.` | - | `.` | - |
| `op.range` | `..` | - | - | `..` |
| `op.index.open` | `[` | `[` | `[` | `[` |
| `op.index.close` | `]` | `]` | `]` | `]` |
| `op.pipe` | `\|>` | - | - | - |
| `stmt.assign` | `=` | `=` | `=` | `=` |
| `stmt.let` | `let` | - | - | `let` |
| `stmt.let.mutable` | `mut` | - | - | `mut` |
| `stmt.let.annotation` | `:` | - | - | `:` |
| `stmt.if` | `if` | `if` | `if` | `if` |
| `stmt.elif` | - | `elif` | `elseif` | - |
| `stmt.else` | `else` | `else` | `else` | `else` |
| `stmt.while` | `while` | `while` | `while` | `while` |
| `stmt.until` | `until` | - | - | - |
| `stmt.for` | `for` | `for` | - | `for` |
| `stmt.for.in` | `in` | `in` | - | `in` |
| `stmt.foreach` | - | - | - | - |
| `stmt.foreach.as` | - | - | - | - |
| `stmt.foreach.pair` | - | - | - | - |
| `stmt.return` | `return` | `return` | `return` | `return` |
| `stmt.break` | `break` | `break` | `break` | `break` |
| `stmt.continue` | `continue` | `continue` | `continue` | `continue` |
| `stmt.function` | `fn` | `def` | `function` | `fn` |
| `stmt.function.returns` | - | - | - | `->` |
| `stmt.pass` | - | `pass` | - | - |
| `stmt.emit` | - | - | - | - |
| `builtin.emit` | `emit` | - | - | - |
| `builtin.print` | - | `print` | `print` | `println!` |
| `builtin.write` | - | - | - | `print!` |
| `builtin.len` | `len` | `len` | `count` `strlen` | `len` |
| `builtin.char_at` | `char_at` | - | - | - |
| `builtin.ord` | `ord` | `ord` | `ord` | - |
| `builtin.chr` | `chr` | `chr` | `chr` | - |
| `builtin.typeof` | `kind` | `type` | `gettype` | - |
| `builtin.error` | `error` | - | - | `panic!` |
| `builtin.extern` | `extern` | - | - | - |
| `builtin.range` | - | `range` | `range` | - |
| `builtin.real` | `real` | - | - | - |
| `builtin.int_to_string` | `int_to_string` | - | - | - |
| `builtin.real_to_string` | `real_to_string` | - | - | - |
| `builtin.rational_to_string` | `rational_to_string` | - | - | - |
| `builtin.bool_to_string` | `bool_to_string` | - | - | - |
| `builtin.array_to_string` | `array_to_string` | - | - | - |
| `builtin.null_to_string` | `null_to_string` | - | - | - |
| `builtin.kind_to_string` | `kind_to_string` | - | - | - |
| `builtin.num` | `num` | - | - | - |
| `builtin.den` | `den` | - | - | - |
| `builtin.int` | `int` | - | - | - |
| `builtin.frac` | `frac` | - | - | - |
| `builtin.push` | `push` | - | - | - |
| `system.args` | `ARGS` | - | `$argv` | - |
| `system.memoization` | `MEMOIZATION` | - | - | - |
| `system.real_default_precision` | `REAL_DEFAULT_PRECISION` | - | - | - |
| `system.entry` | - | - | - | `main` |
| `system.kind.integer` | `INTEGER` | - | - | - |
| `system.kind.rational` | `RATIONAL` | - | - | - |
| `system.kind.real` | `REAL` | - | - | - |
| `system.kind.string` | `STRING` | - | - | - |
| `system.kind.boolean` | `BOOLEAN` | - | - | - |
| `system.kind.array` | `ARRAY` | - | - | - |
| `system.kind.null` | `NULL` | - | - | - |

Operator precedence, lowest tier first. Unary operators sit in their own tier.

- **lumen**: `|>` < `or` < `and` < `==` `!=` `<` `>` `<=` `>=` < `..` < `+` `-` < `*` `/` `%` `//` `.` < `**` < `-` `not` `!`
- **python**: `or` < `and` < `not` < `==` `!=` `<` `>` `<=` `>=` < `+` `-` < `*` `/` `//` `%` < `-` < `**`
- **php**: `or` < `and` < `||` < `&&` < `==` `!=` < `<` `>` `<=` `>=` < `.` < `+` `-` < `*` `/` `%` < `!` < `-` < `**`
- **rust**: `..` < `||` < `&&` < `==` `!=` `<` `>` `<=` `>=` < `+` `-` < `*` `/` `%` < `-` `!`
<!-- table:end -->
