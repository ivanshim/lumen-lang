# Real rendering fixtures

The inherited expectations changed with binary real reading and shortest
rendering:

- `scratch/numbers/1.out`: `1E-5` prints as `1e-05`.
- `scratch/numbers/12.out`: use exponent notation at the real rendering
  boundaries, `inf` on overflow, `0.0` on underflow, `5e-324` for the least
  positive subnormal, and `0.30000000000000004` for `0.1 + 0.2`.
- `scratch/numbers/22.out`: binary arithmetic makes both comparisons false;
  real division retains `.0`, and the long literal rounds to binary precision.
- `scratch/file-str/5.out`: the binary value of `1.e+49` is not the exact
  integer `10 ** 49`, so the first comparison is false.
- `scratch/builtins-core/6.out`: round the binary value of `2.675` to `2.67`,
  retain the sign and real kind of rounded zero, round halfway cases to even,
  and preserve integer results when rounding integers to negative places.

`6.py` also checks even rounding with both signs, integer and real results,
and positive and negative decimal places.

The earlier parameters fixture correction is already present:
`scratch/params/15.err` became `scratch/params/15.out` because
`print(1, end="")` succeeds and writes `1` without a newline. It needs no
further change here.
