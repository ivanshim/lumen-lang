# File grammar

`0.py` takes the continued sum from `test_backslash` in the grammar
suite. It must print `2`. The base reader stops at the backslash;
`claude/py-lexical` owns the full account of continued lines.

The continued-line reading is taken from that branch, under its own
`ext.lexical.line_continuation` label (a list spelling `\`). No numeric
reading or other lexical work from that branch is brought in. The
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
from the lexical branch, without its numeric scanner. The existing
`ext.op.bit.and`, `.left`, `.not`, `.right`, and `.xor` lists spell
`&`, `<<`, `~`, `>>`, and `^`; `.or` already spelled `|`.
`ext.op.bit.whole` is the switch keeping arbitrary-width whole numbers
and refusing text or reals. `6.py` reaches that refusal through
`ext.system.fault.operands`, spelling `unsupported operand type(s)`.
`7.py` reaches `ext.system.fault.shift`, spelling `negative shift count`.
The coordinator should retain the lexical branch's complete account.
