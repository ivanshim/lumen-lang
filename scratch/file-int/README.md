# Integer signs

`0.py` takes the complement from the unary tests. It must fail in the
reader before the bit signs are spelled, and must then say minus three.

`1.py` gives unary plus its existing spelling, `ext.op.plus`, beside
negation and powers. `ext.op.plus.non_number` refuses values that are no
numbers; `10.py` asks for that refusal. Flags become whole numbers.

`2.py` joins bits by `ext.op.bit.and`, `ext.op.bit.or` and
`ext.op.bit.xor`, with either sign and with flags. `3.py` moves them by
`ext.op.bit.left` and `ext.op.bit.right`. `4.py` turns and joins numbers
longer than a machine word with `ext.op.bit.not` and the same signs.
`ext.op.bit.whole` keeps all their bits, sign and all.

`5.py` checks their precedence and compound writes. `6.py` asks for a
negative shift, whose words are `ext.system.fault.shift`. `7.py` and
`8.py` ask for bits from a real and from text; these are refused with
`ext.system.fault.operands`. `9.py` asks for a left shift too long
for the host to hold, named by `ext.op.bit.whole.room`.

Classes, tuples, string prefixes and compound statement forms stand in
the first-wave pieces. These programs do not supply those readers.

The whole-bit reading and running are taken from the lexical piece,
using its `ext.op.bit.whole` label. The room complaint and the unary
plus check are refinements here; the merge should keep that one account
of whole-bit operations.

`11.py` reads binary and octal prefixes under their existing labels,
uppercase hexadecimal through the core prefix list, and underscores
through `ext.lexical.number.separator`. The lexical piece's
`ext.lexical.number.separator.after_prefix` admits a separator straight
after a prefix and checks that other separators have digits on both
sides. `12.py` asks for its `ext.lexical.number.amiss` complaint.

`13.py` takes `ext.lexical.line_continuation` and its scanner from the
lexical piece, so an integer expression may carry on after a backslash.
The same spelling carries a quoted string across a line end. This is
the small reading needed after the bit signs expose the continued line
in the long-number tests; the merge should keep the lexical account.
