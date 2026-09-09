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
