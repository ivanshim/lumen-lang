# Language definitions

One JSON file per language. Each file maps a fixed set of labels, the names
of things the kernel can do, to the strings a language spells them with. The
semantics behind every label belong to the kernel and are the same for every
language: a definition changes how a program is spelled, never what it means.

Both kernels read these files, each in its own way. The microcode kernel
reduces a definition to tables and an instruction tree; the stream kernel
registers handlers for the constructs a definition spells and transforms
the token stream by its block style and markers. The definitions in this
directory (Lumen, RPLumen, Python, Rust) are embedded at build time and picked by
file extension, `--lang <name>` or `--lang <extension>`; the ones in
`extras/` (PHP, Ruby, Pascal, C, JavaScript, Swift) are never compiled
in and are read from disk with `--lang extras/<name>.json`, the same path
any definition of your own takes. Every example runs on both kernels, and
`scripts/kernel_diff.sh` requires them to print the same thing.

These files replace the earlier YAML specifications, EBNF grammars and
grammar documents: a language's surface syntax is its definition, its
semantics are the kernel's, and the table below is the comparison.

## The floor

Every kernel has a floor: the operations that a program cannot express in
terms of the others, because they reach into a value's representation,
have an effect, or are syntax. Everything above the floor is derived, in
the kernel by the derivation stated here or in `lib_lumen/` as Lumen
code, and both kernels derive the same things the same way. A definition
only names what its language spells; the floor is the same for all.

| Domain | Floor | Derived from it |
|---|---|---|
| Integers | `+`, `-`, `*`, `//` (truncating), `<`, `==`, and `/` making a rational | `%` is `a - b * (a // b)`; `**` is multiplication by squaring on the exponent's integer part; `-x` is `0 - x`; `a > b` is `b < a`, `a <= b` is `not b < a`, `a >= b` is `not a < b`, `!=` is `not ==` |
| Rationals and reals | the same operations promoted, `num`, `den`, `real(x, p)`, `precision` | `int(x)` is `num(x) // den(x)`; `frac(x)` is `x - int(x)`; every renderer (`lib_lumen/render.lm`) |
| Strings | `len`, `char_at`, `.`, `ord`, `chr`, `emit` | every function in `string.lm`, `string_ord_chr.lm`, `string_to_value.lm`, and `print`, `write` |
| Arrays | literal, `a[i]`, `a[i] = v`, `len`, `push` | `array.lm`: concatenation, slicing, search, reversal |
| Booleans | `and`, `or` (short-circuit), `not` | nothing further |
| Control | branch, loop, call, return, break, continue | `until` is `while not`; `for v in a..b` is a counted loop, the range being loop syntax rather than a value; `elif`, `else if` and the pipe are spellings |
| Effects and the boundary | `emit`, `error`, `extern`, `kind`, the system bindings | `to_string`, `to_int` and `to_real` are the one-name conversions other languages have; Lumen derives them in its library |

## Format rules

1. A file is one flat JSON object. Every file carries the same labels in the
   same order; `scripts/lang_table.py` fails if they differ, and the
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
   `op.div.result` says what `op.div` yields: `rational`, Lumen's exact
   `/`, or `real`, the `/` of Python, JavaScript, PHP and Pascal, a real at
   the default precision; `null` when the language has no `op.div`.
5. Keys beginning with `$` are for readers, not the kernels: `$comment`
   explains, and `$library` maps a label the language leaves empty to the
   library function that provides it (Lumen's `print`), which the table
   shows as `(library: print)`.
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
12. A builtin name begins like an identifier and may go on with operator
    characters and further words, which is how Rust's `println!` and
    JavaScript's `console.log` are names; the lexers read such a name whole.
13. `print` and `write` join their arguments with single spaces. When the
    first argument is a string holding a placeholder from
    `builtin.print.placeholder` (Rust's `{}`, C's `%d`) and more arguments
    follow, the placeholders are filled in order instead.
14. `system.entry` names a function (Rust's `main`) that the kernel calls
    after the program body if the body defined it.
15. Statement forms are fixed by the kernel: `stmt.let` introduces a binding
    with an optional `stmt.let.mutable` word and an optional
    `stmt.let.annotation` token followed by one type word; `stmt.if` takes a
    condition and a block, then any number of `stmt.elif` branches and one
    `stmt.else`; `stmt.while` and `stmt.until` take a condition and a block,
    the latter testing after each pass; `stmt.for` binds a variable over a
    range with `stmt.for.in`, the range being loop syntax, `start op.range
    end` or `builtin.range(start, end)`, not a value (a postfix language
    leaves `stmt.for.in` empty: the bounds come off the stack); `stmt.function` takes a name, a parenthesised
    parameter list whose entries may carry the binding annotation, an
    optional `stmt.function.returns` token with one type word, and a block.
    A binding with no value (Pascal's `var x: integer;`, Rust's `let x;`)
    binds the null value. A line end or a `stmt.terminator` ends
    a statement in every block style, except inside brackets and
    parentheses, so Swift writes no semicolons and Rust may.
16. `block.style` picks how a block is delimited. `indentation`: a block
    begins after an optional `block.intro` token at the end of the line and
    runs while lines are indented by `block.indent_size`. `braces`: a block
    runs from a `block.open` word to the `block.close` word paired with it
    by position (`{` with `}`, `begin` with `end`), and a bare block is a
    statement that runs in its own scope. `keyword`: a block begins after
    the statement head, optionally after a `block.intro` word (`then`,
    `do`), and runs to a `block.close` word (`end`); `block.open` is empty,
    and one `end` closes a whole `if`/`elif`/`else` chain. `postfix`: there
    are no statements or expressions, only words acting on one stack (rule
    24); a body runs from the control word to a `block.close` word, with
    `block.intro` words inside (`while` cond `repeat` body `end`).
17. Paired lists are parallel: the n-th entry of `block.open` closes with
    the n-th of `block.close`, and the n-th of `lexical.comment_block.open`
    with the n-th of `lexical.comment_block.close`, so a language may have
    several comment forms (Pascal's `{ }` and `(* *)`).
18. `stmt.let.type_first` makes the `stmt.let` words type names that come
    first, as in C: `int x = 0;` binds, `int x;` binds null, and a name
    followed by the call bracket, `long fib(int n) { ... }`, defines a
    function whose parameters are a type word and a name each, a type word
    alone (`void`) declaring none. With it, `stmt.let.mutable`,
    `stmt.let.annotation`, `stmt.function` and `stmt.function.returns`
    stay empty.
19. `syntax.call.label` is the token after an argument label in a call,
    Swift's `fib(n: 10)`. The label names the argument for the reader;
    arguments pass by position and the label is dropped.
20. The pipe passes the value on its left as the first argument of the
    call on its right, and a bare name on the right is a call with no other
    arguments. Spelled `.` in the highest tier, that is method syntax:
    `arr.push(x)` is `push(arr, x)`, `s.length` is `len(s)`, `42.to_s` is
    `to_string(42)`. The kernels accept both forms; a language lists the
    names it has, and writes them the way it does.
21. `op.index.strings` lets `s[i]` index a string, yielding the character
    at that position, in languages that spell `char_at` that way.
22. A function whose header ends with a terminator (Pascal's
    `function f(a: integer; b: real): integer;`) may be followed by
    declarations before its body block, and its typed parameter groups are
    separated by the terminator. With `stmt.function.result_by_name`, a
    body that ends without returning yields what it assigned to the
    function's own name; `stmt.return` (Pascal's `exit`) still returns
    early, with or without a value.
23. The renderers are not kernel builtins. `precision` reads the
    significant digits a real carries, `num` and `den` read the fraction
    any number is, and Lumen's library derives `int` and `frac` from them
    (`lib_lumen/numeric.lm`) and renders every value with `kind`, `num`,
    `den`, `precision`, `len` and `char_at` (`lib_lumen/render.lm`).
    `to_string`, `to_int` and `to_real`
    are the one-name conversions of languages that have them (`str`,
    `String`, `Integer`): any value as `print` shows it, the integer part
    of any number, any number as a real.
24. A postfix language (RPLumen, `langs/rplumen.json`) is Lumen read the
    other way round: `5 3 +` is `5 + 3`. A literal pushes itself, an
    operator pops its operands and pushes the result, so `op.precedence`
    is empty, and a builtin takes its arguments off the stack (`s 1
    char_at`) and pushes its result, if it has one (`x print` pushes
    nothing). A name between `lexical.name_quote` marks (`'x'`) is data
    for the word after it, which must take a name: `8 'x' =` assigns
    (`stmt.assign`, or `stmt.let`), `4 'arr' push` appends, `0 99 'arr'
    put` writes an element (`builtin.put`, and `builtin.get` reads one:
    `arr 2 get`), `0 10 'i' for ... next` counts. A bare word pushes its
    binding's value, or runs it when it holds a program. A program is the
    words between `stack.program.open` and `stack.program.close` (`« ...
    »`, or `<< ... >>`), a value that `stack.eval` or a bare name runs:
    `« 'b' = 'a' = a b + » 'add' =` defines and `2 3 add` calls. Control
    words take their condition from the top of the stack: `cond if ...
    else ... end`, `while cond repeat ... end`, `do ... until cond end`.
    An array literal gathers what its body pushes (`[ 1 2 3 ]`). The stack
    words are RPL's: `stack.dup`, `stack.drop`, `stack.swap`,
    `stack.over`, `stack.rot`. Nothing prints on its own; `print` and
    `write` are kernel builtins here because Lumen's renderers are written
    in infix Lumen.

## The Lumen examples in every language

`scripts/port_examples.py` writes every program under `examples/lumen/` in
each other language, driven by these definitions: a construct is spelled as
the target's labels say, and an example is left out of a language only
when a label it needs is empty there. `examples/PORTS.md` is the result,
example by example. Reading it is a quick way to see what each definition
still lacks: `kind` and the kind names, `extern`, `write` in Swift,
functions in Pascal.

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

Generated by `python3 scripts/lang_table.py`; edit the JSON, not the table.

<!-- table:start -->
| Label | lumen | rplumen | python | rust | c (extra) | javascript (extra) | pascal (extra) | php (extra) | ruby (extra) | swift (extra) |
|---|---|---|---|---|---|---|---|---|---|---|
| `format_version` | `1` | `1` | `1` | `1` | `1` | `1` | `1` | `1` | `1` | `1` |
| `language` | `lumen` | `rplumen` | `python` | `rust` | `c` | `javascript` | `pascal` | `php` | `ruby` | `swift` |
| `extensions` | `lm` | `rpl` | `py` | `rs` | `c` | `js` | `pas` | `php` | `rb` | `swift` |
| `lexical.comment_line` | `#` | `#` | `#` | `//` | `//` `#` | `//` | `//` | `//` `#` | `#` | `//` |
| `lexical.comment_block.open` | - | - | - | `/*` | `/*` | `/*` | `{` `(*` | `/*` | `=begin` | `/*` |
| `lexical.comment_block.close` | - | - | - | `*/` | `*/` | `*/` | `}` `*)` | `*/` | `=end` | `*/` |
| `lexical.string_quotes` | `"` `'` | `"` | `"` `'` | `"` | `"` | `"` `'` | `'` | `"` `'` | `"` `'` | `"` |
| `lexical.raw_quotes` | `'` | - | - | - | - | - | `'` | `'` | `'` | - |
| `lexical.string_escapes` | `n` `t` `\` `"` | `n` `t` `\` `"` | `n` `t` `\` `"` `'` | `n` `t` `\` `"` | `n` `t` `\` `"` | `n` `t` `\` `"` `'` | - | `n` `t` `\` `"` | `n` `t` `\` `"` | `n` `t` `\` `"` |
| `lexical.prologue` | - | - | `import sys` | - | - | - | - | `<?php` | - | - |
| `lexical.name_quote` | - | `'` | - | - | - | - | - | - | - | - |
| `lexical.number.decimal_point` | `.` | `.` | `.` | `.` | `.` | `.` | `.` | `.` | `.` | `.` |
| `lexical.number.base_marker` | `@` | `@` | - | - | - | - | - | - | - | - |
| `lexical.number.exponent_marker` | `^` | `^` | - | - | - | - | - | - | - | - |
| `lexical.number.hex_prefix` | - | - | `0x` | `0x` | `0x` | `0x` | - | `0x` | `0x` | `0x` |
| `lexical.keywords_case_insensitive` | `false` | `false` | `false` | `false` | `false` | `false` | `true` | `true` | `false` | `false` |
| `identifier.unicode` | `true` | `true` | `true` | `true` | `true` | `true` | `true` | `true` | `true` | `true` |
| `identifier.variable_prefix` | - | - | - | - | - | - | - | `$` | - | - |
| `identifier.case_insensitive` | `false` | `false` | `false` | `false` | `false` | `false` | `true` | `false` | `false` | `false` |
| `block.style` | `indentation` | `postfix` | `indentation` | `braces` | `braces` | `braces` | `braces` | `braces` | `keyword` | `braces` |
| `block.open` | - | - | - | `{` | `{` | `{` | `begin` | `{` | - | `{` |
| `block.close` | - | `end` `next` | - | `}` | `}` | `}` | `end` | `}` | `end` | `}` |
| `block.intro` | - | `repeat` `until` | `:` | - | - | - | `then` `do` | - | `then` `do` | - |
| `block.indent_size` | `4` | - | `4` | - | - | - | - | - | - | - |
| `stmt.terminator` | `;` | - | - | `;` | `;` | `;` | `;` `.` | `;` | - | - |
| `syntax.group.open` | `(` | - | `(` | `(` | `(` | `(` | `(` | `(` | `(` | `(` |
| `syntax.group.close` | `)` | - | `)` | `)` | `)` | `)` | `)` | `)` | `)` | `)` |
| `syntax.call.open` | `(` | - | `(` | `(` | `(` | `(` | `(` | `(` | `(` | `(` |
| `syntax.call.separator` | `,` | - | `,` | `,` | `,` | `,` | `,` | `,` | `,` | `,` |
| `syntax.call.close` | `)` | - | `)` | `)` | `)` | `)` | `)` | `)` | `)` | `)` |
| `syntax.call.label` | - | - | - | - | - | - | - | - | - | `:` |
| `syntax.array.open` | `[` | `[` | `[` | `[` | - | `[` | - | `[` | `[` | `[` |
| `syntax.array.separator` | `,` | - | `,` | `,` | - | `,` | - | `,` | `,` | `,` |
| `syntax.array.close` | `]` | `]` | `]` | `]` | - | `]` | - | `]` | `]` | `]` |
| `syntax.map.open` | - | - | - | - | - | - | - | - | - | - |
| `syntax.map.separator` | - | - | - | - | - | - | - | - | - | - |
| `syntax.map.pair` | - | - | - | - | - | - | - | - | - | - |
| `syntax.map.close` | - | - | - | - | - | - | - | - | - | - |
| `literal.true` | `true` | `true` | `True` | `true` | `true` | `true` | `true` | `true` | `true` | `true` |
| `literal.false` | `false` | `false` | `False` | `false` | `false` | `false` | `false` | `false` | `false` | `false` |
| `literal.null` | `null` | `null` | `None` | `None` | `NULL` | `null` `undefined` | `nil` | `null` | `nil` | `nil` |
| `op.right_associative` | `**` | - | `**` | - | - | `**` | - | `**` | `**` | - |
| `op.add` | `+` | `+` | `+` | `+` | `+` | `+` | `+` | `+` | `+` | `+` |
| `op.sub` | `-` | `-` | `-` | `-` | `-` | `-` | `-` | `-` | `-` | `-` |
| `op.mul` | `*` | `*` | `*` | `*` | `*` | `*` | `*` | `*` | `*` | `*` |
| `op.div` | `/` | `/` | `/` | - | - | `/` | `/` | `/` | - | - |
| `op.div.result` | `rational` | `rational` | `real` | - | - | `real` | `real` | `real` | - | - |
| `op.quot` | `//` | `//` | `//` | `/` | `/` | - | `div` | - | `/` | `/` |
| `op.rem` | `%` | `%` | `%` | `%` | `%` | `%` | `mod` | `%` | `%` | `%` |
| `op.pow` | `**` | `**` | `**` | - | - | `**` | - | `**` | `**` | - |
| `op.eq` | `==` | `==` | `==` | `==` | `==` | `===` `==` | `=` | `==` | `==` | `==` |
| `op.ne` | `!=` | `!=` | `!=` | `!=` | `!=` | `!==` `!=` | `<>` | `!=` | `!=` | `!=` |
| `op.lt` | `<` | `<` | `<` | `<` | `<` | `<` | `<` | `<` | `<` | `<` |
| `op.le` | `<=` | `<=` | `<=` | `<=` | `<=` | `<=` | `<=` | `<=` | `<=` | `<=` |
| `op.gt` | `>` | `>` | `>` | `>` | `>` | `>` | `>` | `>` | `>` | `>` |
| `op.ge` | `>=` | `>=` | `>=` | `>=` | `>=` | `>=` | `>=` | `>=` | `>=` | `>=` |
| `op.and` | `and` | `and` | `and` | `&&` | `&&` | `&&` | `and` | `&&` `and` | `&&` `and` | `&&` |
| `op.or` | `or` | `or` | `or` | `\|\|` | `\|\|` | `\|\|` | `or` | `\|\|` `or` | `\|\|` `or` | `\|\|` |
| `op.not` | `not` `!` | `not` | `not` | `!` | `!` | `!` | `not` | `!` | `!` `not` | `!` |
| `op.negate` | `-` | `neg` | `-` | `-` | `-` | `-` | `-` | `-` | `-` | `-` |
| `op.concat` | `.` | `.` | - | - | - | - | - | `.` | - | - |
| `op.range` | `..` | - | - | `..` | - | - | - | - | `...` | `..<` |
| `op.index.open` | `[` | - | `[` | `[` | `[` | `[` | `[` | `[` | `[` | `[` |
| `op.index.close` | `]` | - | `]` | `]` | `]` | `]` | `]` | `]` | `]` | `]` |
| `op.index.strings` | `false` | `false` | `true` | `false` | `false` | `true` | `false` | `true` | `true` | `false` |
| `op.pipe` | `\|>` | - | `.` | `.` | - | `.` | - | - | `.` | `.` |
| `stmt.assign` | `=` | `=` | `=` | `=` | `=` | `=` | `:=` | `=` | `=` | `=` |
| `stmt.let` | `let` | `let` | - | `let` | `int` `long` `double` `float` `bool` `void` | `let` `const` `var` | `var` | - | - | `let` `var` |
| `stmt.let.mutable` | `mut` | - | - | `mut` | - | - | - | - | - | - |
| `stmt.let.annotation` | `:` | - | - | `:` | - | - | `:` | - | - | `:` |
| `stmt.let.type_first` | `false` | `false` | `false` | `false` | `true` | `false` | `false` | `false` | `false` | `false` |
| `stmt.if` | `if` | `if` | `if` | `if` | `if` | `if` | `if` | `if` | `if` | `if` |
| `stmt.elif` | - | - | `elif` | - | - | - | - | `elseif` | `elsif` | - |
| `stmt.else` | `else` | `else` | `else` | `else` | `else` | `else` | `else` | `else` | `else` | `else` |
| `stmt.while` | `while` | `while` | `while` | `while` | `while` | `while` | `while` | `while` | `while` | `while` |
| `stmt.until` | `until` | `do` | - | - | - | - | - | - | - | - |
| `stmt.for` | `for` | `for` | `for` | `for` | - | - | - | - | `for` | `for` |
| `stmt.for.in` | `in` | - | `in` | `in` | - | - | - | - | `in` | `in` |
| `stmt.foreach` | - | - | - | - | - | - | - | - | - | - |
| `stmt.foreach.as` | - | - | - | - | - | - | - | - | - | - |
| `stmt.foreach.pair` | - | - | - | - | - | - | - | - | - | - |
| `stmt.return` | `return` | `return` | `return` | `return` | `return` | `return` | `exit` | `return` | `return` | `return` |
| `stmt.break` | `break` | `break` | `break` | `break` | `break` | `break` | `break` | `break` | `break` | `break` |
| `stmt.continue` | `continue` | `continue` | `continue` | `continue` | `continue` | `continue` | `continue` | `continue` | `next` | `continue` |
| `stmt.function` | `fn` | - | `def` | `fn` | - | `function` | `function` `procedure` | `function` | `def` | `func` |
| `stmt.function.returns` | - | - | - | `->` | - | - | `:` | - | - | `->` |
| `stmt.function.result_by_name` | `false` | `false` | `false` | `false` | `false` | `false` | `true` | `false` | `false` | `false` |
| `stmt.pass` | - | - | `pass` | - | - | - | - | - | - | - |
| `stmt.emit` | - | - | - | - | - | - | - | - | - | - |
| `stack.dup` | - | `dup` | - | - | - | - | - | - | - | - |
| `stack.drop` | - | `drop` | - | - | - | - | - | - | - | - |
| `stack.swap` | - | `swap` | - | - | - | - | - | - | - | - |
| `stack.over` | - | `over` | - | - | - | - | - | - | - | - |
| `stack.rot` | - | `rot` | - | - | - | - | - | - | - | - |
| `stack.eval` | - | `eval` | - | - | - | - | - | - | - | - |
| `stack.program.open` | - | `«` `<<` | - | - | - | - | - | - | - | - |
| `stack.program.close` | - | `»` `>>` | - | - | - | - | - | - | - | - |
| `builtin.emit` | `emit` | `emit` | `sys.stdout.write` | - | - | `process.stdout.write` | - | - | - | - |
| `builtin.print` | (library: `print`) | `print` | `print` | `println!` | `puts` | `console.log` | `writeln` | - | `puts` | `print` |
| `builtin.write` | (library: `write`) | `write` | - | `print!` | `printf` | - | `write` | `print` | `print` | - |
| `builtin.print.placeholder` | - | - | - | `{}` | `%d` `%ld` `%f` `%lf` `%s` `%c` | - | - | - | - | - |
| `builtin.len` | `len` | `len` | `len` | `len` | `strlen` | `length` | `length` | `count` `strlen` | `length` `size` | `count` |
| `builtin.char_at` | `char_at` | `char_at` | - | - | - | `charAt` | - | - | - | - |
| `builtin.ord` | `ord` | `ord` | `ord` | - | - | - | `ord` | `ord` | `ord` | - |
| `builtin.chr` | `chr` | `chr` | `chr` | - | - | `String.fromCharCode` | `chr` | `chr` | `chr` | - |
| `builtin.typeof` | `kind` | `kind` | `type` | - | - | - | - | `gettype` | - | - |
| `builtin.error` | `error` | `error` | `sys.exit` | `panic!` | - | - | - | `exit` | `raise` | `fatalError` |
| `builtin.extern` | `extern` | - | - | - | - | - | - | - | - | - |
| `builtin.range` | - | - | `range` | - | - | - | - | `range` | - | - |
| `builtin.real` | `real` | `real` | - | - | - | - | - | - | - | - |
| `builtin.num` | `num` | `num` | - | - | - | - | - | - | - | - |
| `builtin.den` | `den` | `den` | - | - | - | - | - | - | - | - |
| `builtin.push` | `push` | `push` | `append` | `push` | - | `push` | - | `array_push` | `push` | `append` |
| `builtin.get` | - | `get` | - | - | - | - | - | - | - | - |
| `builtin.put` | - | `put` | - | - | - | - | - | - | - | - |
| `builtin.precision` | `precision` | `precision` | - | - | - | - | - | - | - | - |
| `builtin.to_string` | (library: `value_to_string`) | `to_string` | `str` | `to_string` | - | `String` | - | `strval` | `to_s` `String` | `String` |
| `builtin.to_int` | (library: `int`) | `to_int` | `int` | - | - | `Math.trunc` | - | `intval` | `to_i` `Integer` | `Int` |
| `builtin.to_real` | (library: `real_default`) | `to_real` | `float` | - | - | `Number` | - | `floatval` | `to_f` `Float` | `Double` |
| `system.args` | `ARGS` | - | - | - | - | - | - | `$argv` | - | - |
| `system.memoization` | `MEMOIZATION` | - | - | - | - | - | - | - | - | - |
| `system.real_default_precision` | `REAL_DEFAULT_PRECISION` | - | - | - | - | - | - | - | - | - |
| `system.entry` | - | - | - | `main` | `main` | - | - | - | - | - |
| `system.kind.integer` | `INTEGER` | `INTEGER` | - | - | - | - | - | - | - | - |
| `system.kind.rational` | `RATIONAL` | `RATIONAL` | - | - | - | - | - | - | - | - |
| `system.kind.real` | `REAL` | `REAL` | - | - | - | - | - | - | - | - |
| `system.kind.string` | `STRING` | `STRING` | - | - | - | - | - | - | - | - |
| `system.kind.boolean` | `BOOLEAN` | `BOOLEAN` | - | - | - | - | - | - | - | - |
| `system.kind.array` | `ARRAY` | `ARRAY` | - | - | - | - | - | - | - | - |
| `system.kind.null` | `NULL` | `NULL` | - | - | - | - | - | - | - | - |

Operator precedence, lowest tier first. Unary operators sit in their own tier.

- **lumen**: `|>` < `or` < `and` < `==` `!=` `<` `>` `<=` `>=` < `..` < `+` `-` < `*` `/` `%` `//` `.` < `**` < `-` `not` `!`
- **rplumen**: 
- **python**: `or` < `and` < `not` < `==` `!=` `<` `>` `<=` `>=` < `+` `-` < `*` `/` `//` `%` < `-` < `**` < `.`
- **rust**: `..` < `||` < `&&` < `==` `!=` `<` `>` `<=` `>=` < `+` `-` < `*` `/` `%` < `-` `!` < `.`
- **c (extra)**: `||` < `&&` < `==` `!=` < `<` `>` `<=` `>=` < `+` `-` < `*` `/` `%` < `!` `-`
- **javascript (extra)**: `||` < `&&` < `===` `!==` `==` `!=` < `<` `>` `<=` `>=` < `+` `-` < `*` `/` `%` < `!` `-` < `**` < `.`
- **pascal (extra)**: `=` `<>` `<` `>` `<=` `>=` < `+` `-` `or` < `*` `/` `div` `mod` `and` < `-` `not`
- **php (extra)**: `or` < `and` < `||` < `&&` < `==` `!=` < `<` `>` `<=` `>=` < `.` < `+` `-` < `*` `/` `%` < `!` < `-` < `**`
- **ruby (extra)**: `or` < `and` < `not` < `||` < `&&` < `==` `!=` < `<` `>` `<=` `>=` < `...` < `+` `-` < `*` `/` `%` < `-` < `!` < `**` < `.`
- **swift (extra)**: `||` < `&&` < `==` `!=` < `<` `>` `<=` `>=` < `..<` < `+` `-` < `*` `/` `%` < `!` `-` < `.`
<!-- table:end -->
