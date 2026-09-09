# File grammar

`0.py` takes the continued sum from `test_backslash` in the grammar
suite. It must print `2`. The base reader stops at the backslash;
the lexical piece owns the full account of continued lines.

The continued-line reading is taken from that branch, under its own
`ext.lexical.line_continuation` label (a list spelling `\`). The
numeric reading brought in beside it is described below. The
coordinator should keep its fuller account when the branches meet.

`1.py` carries a string across a line end and leaves a backslash in a
comment alone. It prints `ab` and `0`; the same continuation label
governs the escaped line end in ordinary strings.

`2.py` reads matrix products and compound matrix writes in an uncalled
routine, then prints `read`. `3.py` and `4.py` reach each form and stop
with `NotImplementedError: matrix multiplication is not supported`.
The product label is `ext.op.matrix` (a list spelling `@`), and its
complaint is `ext.op.matrix.unready` (a list of those plain words).
The product methods are not yet called; these two complaints mark that
limit of this stage rather than claiming a scalar product has a value.

`5.py` takes the six bit operators from the grammar suite and prints
`1`, `0`, `1`, `2`, `0`, and `-2`. Their whole-number reading is taken
from the lexical branch. The existing
`ext.op.bit.and`, `.left`, `.not`, `.right`, and `.xor` lists spell
`&`, `<<`, `~`, `>>`, and `^`; `.or` already spelled `|`.
`ext.op.bit.whole` is the switch keeping arbitrary-width whole numbers
and refusing text or reals. `6.py` reaches that refusal through
`ext.system.fault.operands`, spelling `unsupported operand type(s)`.
`7.py` reaches `ext.system.fault.shift`, spelling `negative shift count`.
The coordinator should retain the lexical branch's complete account.

`8.py` reads nested classes, empty base brackets, several bases, methods,
and a one-line body in an uncalled routine, then prints `read`.
`9.py` reaches a class declaration and stops with
`NotImplementedError: this class form cannot run yet`.
`ext.stmt.class` is the existing list spelling `class`;
`ext.stmt.class.unready` takes its spelling from the class branch.
This is only the small reading needed to reach what follows a class;
all class execution remains for that branch, whose fuller reading
should replace this one when the coordinator brings the pieces together.

`10.py` covers the grammar suite's base prefixes, separators, decimal
points with one side empty, and exponents. It prints `255`, `255`, `9`,
`1000`, then four true comparisons. The numeric reading comes from the
lexical branch; its printing changes remain there. The existing
`ext.lexical.number.binary_prefix`, `.octal_prefix`, `.exponent`,
`.separator`, and `.amiss` lists spell `0b`/`0B`, `0o`/`0O`, `e`/`E`,
`_`, and `invalid numeric literal`. The `.point.bare` and
`.separator.after_prefix` switches are true. The core hex prefix also
spells `0X`. `11.py` refuses a doubled separator in a whole number.

`12.py` reads empty, single, trailing-comma, and nested tuples, and
a loop over values joined without brackets, in an uncalled routine.
It prints `read`. `13.py` reaches a tuple and says
`NotImplementedError: tuple values are not supported`.
The existing `ext.op.tuple` list spells `,`; its `.unready` list holds
those words. This is the small read needed by the grammar file; the
tuple piece remains responsible for tuple values and taking them apart.

The string piece supplies the scanning and field reading used here.
Its byte-as-text and unsupported-format fallbacks are not brought in:
`ext.lexical.string.unready` is a list spelling
`NotImplementedError: this string form is not supported`.
`14.py` prints a three-line quoted string. `15.py` prints raw text,
adjacent strings, and numbered characters. `16.py` prints a plain
formatted field, then reads richer fields in an uncalled function and
prints `read`. `17.py` refuses byte text; `18.py` refuses a format
specification. `19.py` refuses a named Unicode escape through
`ext.lexical.escape.unavailable`. These labels and their counts and
spellings are described beside the string paragraphs in the language
definitions. The coordinator should keep the string piece's reading
and these honest limits wherever fuller running remains wanting.

`20.py` reads one manager, several managers, grouped managers and
nested binding names in an uncalled routine, then prints `read`.
`21.py` reaches a context statement and stops with
`NotImplementedError: context managers are not supported`, before
its body can print. The existing block-piece spellings are
`ext.stmt.with` (`with`) and `ext.stmt.with.as` (`as`), both lists;
`ext.stmt.with.unready` holds the complaint as a list. The body is
read whole; entry and exit calls remain for the block piece.

`22.py` reads an ellipsis literal in an uncalled routine and prints
`read`; `23.py` reaches it and stops with
`NotImplementedError: ellipsis values are not supported`. The
expression piece owns the value. Its `ext.literal.ellipsis` list
spells `...`; the `.unready` list supplies these words for this small
reading. An indexed ellipsis keeps the slice piece's own meaning.

## Spellings in this piece

`26.py` reads lambda parameters, nested defaults and bodies, and a lambda
decorator. `27.py` reaches a lambda and gives the explicit value complaint;
`29.py` refuses duplicate parameter names. `28.py` checks that assignment
expressions bind and answer with their value, including in a decorator.
`ext.op.lambda` spells `lambda`, `ext.op.lambda.unready` spells
`NotImplementedError: lambda values are not supported`, and
`ext.op.assign.expression` spells `:=`; all three are lists.

`scratch/annotations/9.py` keeps its former `lambda x: x` in an uncalled
routine and prints `read`, as the reference does. `9.out` replaces the
old colon-reader error. The fixture now checks reading the parameter/body
boundary without requiring lambda values, whose execution belongs to the
expression piece.

`24.py` reads the grammar suite's comma-separated return values, including
a trailing comma, and checks that single and empty returns still answer
`7` and `None`. `25.py` reaches such a tuple return and gives the same
explicit tuple-value complaint as a grouped tuple. The return reader now
uses the existing `ext.op.tuple` spelling; tuple execution remains for its
piece.

The table gives every new or changed spelling against the required base.
Lists, switches and counts retain the definition's types. The operator
tiers are shown whole, lowest first.

| Label | Type | Spelling or value |
|---|---|---|
| `lexical.string_escapes` | list | `["n", "t", "r", "0", "\\", "\"", "'"]` |
| `lexical.number.hex_prefix` | list | `["0x", "0X"]` |
| `op.precedence` | tiers | `[["or"], ["and"], ["not"], ["==", "!=", "<", ">", "<=", ">="], ["\|"], ["^"], ["&"], ["<<", ">>"], ["+", "-"], ["*", "@", "/", "//", "%"], ["-", "~"], ["**"], ["."]]` |
| `ext.lexical.escape.byte` | list | `["x"]` |
| `ext.lexical.escape.byte.digits` | count | `2` |
| `ext.lexical.escape.codepoint` | list | `["u"]` |
| `ext.lexical.escape.codepoint.amiss` | list | `["invalid Unicode escape"]` |
| `ext.lexical.escape.codepoint.beyond` | list | `["Unicode code point out of range"]` |
| `ext.lexical.escape.codepoint.digits` | count | `4` |
| `ext.lexical.escape.codepoint.wide` | list | `["U"]` |
| `ext.lexical.escape.codepoint.wide.digits` | count | `8` |
| `ext.lexical.escape.continued` | switch | `true` |
| `ext.lexical.escape.controls` | list | `["a", "b", "f", "v"]` |
| `ext.lexical.escape.named` | list | `["N"]` |
| `ext.lexical.escape.octal` | switch | `true` |
| `ext.lexical.escape.unavailable` | list | `["Unicode escape cannot be represented"]` |
| `ext.lexical.line_continuation` | list | `["\\"]` |
| `ext.lexical.number.amiss` | list | `["invalid numeric literal"]` |
| `ext.lexical.number.binary_prefix` | list | `["0b", "0B"]` |
| `ext.lexical.number.exponent` | list | `["e", "E"]` |
| `ext.lexical.number.octal_prefix` | list | `["0o", "0O"]` |
| `ext.lexical.number.point.bare` | switch | `true` |
| `ext.lexical.number.separator` | list | `["_"]` |
| `ext.lexical.number.separator.after_prefix` | switch | `true` |
| `ext.lexical.string.adjacent` | switch | `true` |
| `ext.lexical.string.amiss` | list | `["invalid string literal"]` |
| `ext.lexical.string.long` | list | `["\"\"\"", "'''"]` |
| `ext.lexical.string.prefix.bytes` | list | `["b", "B"]` |
| `ext.lexical.string.prefix.format` | list | `["f", "F"]` |
| `ext.lexical.string.prefix.plain` | list | `["u", "U"]` |
| `ext.lexical.string.prefix.raw` | list | `["r", "R"]` |
| `ext.lexical.string.unready` | list | `["NotImplementedError: this string form is not supported"]` |
| `ext.literal.ellipsis` | list | `["..."]` |
| `ext.literal.ellipsis.unready` | list | `["NotImplementedError: ellipsis values are not supported"]` |
| `ext.op.bit.and` | list | `["&"]` |
| `ext.op.assign.expression` | list | `[":="]` |
| `ext.op.lambda` | list | `["lambda"]` |
| `ext.op.lambda.unready` | list | `["NotImplementedError: lambda values are not supported"]` |
| `ext.op.bit.left` | list | `["<<"]` |
| `ext.op.bit.not` | list | `["~"]` |
| `ext.op.bit.right` | list | `[">>"]` |
| `ext.op.bit.whole` | switch | `true` |
| `ext.op.bit.xor` | list | `["^"]` |
| `ext.op.matrix` | list | `["@"]` |
| `ext.op.matrix.unready` | list | `["NotImplementedError: matrix multiplication is not supported"]` |
| `ext.op.tuple` | list | `[","]` |
| `ext.op.tuple.unready` | list | `["NotImplementedError: tuple values are not supported"]` |
| `ext.stmt.class` | list | `["class"]` |
| `ext.stmt.class.unready` | list | `["NotImplementedError: this class form cannot run yet"]` |
| `ext.stmt.with` | list | `["with"]` |
| `ext.stmt.with.as` | list | `["as"]` |
| `ext.stmt.with.unready` | list | `["NotImplementedError: context managers are not supported"]` |
| `ext.system.fault.operands` | list | `["unsupported operand type(s)"]` |
| `ext.system.fault.shift` | list | `["negative shift count"]` |

The borrowed lexical reading is from `a3d8d6c`;
the borrowed string reading is from `ab5ff7c`. The class, tuple, context, and
ellipsis readings here are deliberately small and await their pieces.

`scratch/annotations/10.py` now keeps its annotated class in an uncalled
routine and prints `read`, as the reference does. Its former colon-reader
error ceased to apply once class headers could be read. The fixture still
checks reading a class field annotation without asking this piece to run
a class or resolve its annotation; `10.out` replaces `10.err`.
