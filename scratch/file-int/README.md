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
`ext.op.bit.unbounded` keeps all their bits, sign and all.

`5.py` checks their precedence and compound writes. `6.py` asks for a
negative shift, whose words are `ext.system.fault.shift`. `7.py` and
`8.py` ask for bits from a real and from text; these are refused with
`ext.op.bit.unbounded.operand`. `9.py` asks for a left shift too long
for the host to hold, named by `ext.op.bit.unbounded.room`.

Classes, tuples, string prefixes and compound statement forms stand in
the first-wave pieces. These programs do not supply those readers.
