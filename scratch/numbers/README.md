# Number literals

These programs ask the two full kernels to read the new number spellings.
Their expected output follows the six-kernel rule: a spelling may be new,
but the worth of an old number and the manner of writing it must remain
as before. The four reference kernels do not yet read every new spelling.

The `.out` files therefore give the kernels' own writing. CPython differs
in these places:

| Program | Kernels | CPython |
|---|---|---|
| `1.py`, `4.py` | Whole reals have no `.0`; `1E-5` is `0.00001`. | Whole reals keep `.0`; `1E-5` is `1e-05`. |
| `12.py` | `1e15` and `1e16` are written out in full, without a point. | `1000000000000000.0` and `1e+16`. |
| `12.py` | `1e309` is one followed by 309 noughts. | `inf`. |
| `12.py` | `1e-400` and `5e-324` are held as nonzero reals, but each is shown as `0.00000000000000` at the usual precision. | `0.0` and `5e-324`. |
| `12.py` | `-0.` is shown as `-0`; `0.1 + 0.2` is `0.3`. | `-0.0` and `0.30000000000000004`. |
| `22.py` | Both equalities are true; the quotient is `5` and the long decimal keeps its written figures. | Both equalities are false; the quotient is `5.0` and the decimal is `1.2345678901234567`. |

`22.py` uses only the old spellings, lest binary rounding or a new manner
of writing reals return unnoticed. Imaginary arithmetic remains unfinished;
the `.err` files which say so are deliberate, though CPython can do that work.
