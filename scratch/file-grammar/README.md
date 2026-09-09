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
