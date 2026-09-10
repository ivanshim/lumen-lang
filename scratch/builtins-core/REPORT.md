# Core builtins compatibility

The Python reader keeps the shared library's rounding behavior. Halfway
values round away from zero: `round(2.5)` is `3`, and `round(-2.5)` is `-3`.
CPython instead rounds ties to even. Decimal places follow the library's
scaling arithmetic, so `round(2.675, 2)` is `2.68`. Negative decimal counts
act as zero places, so `round(125, -1)` is `125`, rather than CPython's `120`.
The `ndigits` argument uses these same rules. The numeric scratch fixture
now expects the library results and also covers named arguments and values
that round to zero. The examples and reference suites remain unchanged.

The parameters fixture `scratch/params/15.py` calls `print(1, end="")`.
Commit `06135178` already replaced `scratch/params/15.err`, which expected
an unsupported-keyword refusal, with `scratch/params/15.out`, containing
exactly `1` without a trailing newline. This is CPython's true output:
`end` is supported, so the old refusal no longer describes the behavior.
That correction is retained; no other parameters fixtures are changed here.

Builds and program execution are performed only in the remote CI jobs.
