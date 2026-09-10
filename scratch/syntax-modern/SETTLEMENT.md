# Modern syntax and lexical closures

The closure rules follow Python's [name resolution](https://docs.python.org/3/reference/executionmodel.html#resolution-of-names),
[nonlocal statements](https://docs.python.org/3/reference/simple_stmts.html#the-nonlocal-statement),
and [function defaults](https://docs.python.org/3/reference/compound_stmts.html#function-definitions).
Free variables are read at call time; defaults retain the values evaluated
when the function is defined. Collection aliases retain the collection even
when a captured binding is rebound or deleted.

## Changed existing expectations

Paths below are relative to `scratch/`. An `.err` to `.out` change removes
the old error file and adds the specified output file. No source program
in this list was changed.

| Fixture | Change and reason |
|---|---|
| `syntax-modern/8.err` → `8.out` | Empty successful output: valid `nonlocal x` assigns the enclosing binding. |
| `syntax-modern/16.err` | The deleted free variable raises the specific enclosing-scope `NameError`, rather than a generic undefined-variable diagnostic; the global `x` must not be used. |
| `syntax-modern/17.err` | Module-level `nonlocal` is a `SyntaxError`, not an unsupported feature. |
| `expressions/8.err` → `8.out` | Empty successful output: `lambda *, x=3: x` accepts its keyword-only default; the result is not printed. |
| `file-builtin/15.err` → `15.out` | Empty successful output: a valid enclosing `nonlocal` declaration needs no refusal. |
| `file-exceptions/7.err` → `7.out` | Empty successful output: valid `nonlocal x` reassigns the enclosing binding. |
| `file-generators/10.err` → `10.out` | Empty successful output: valid `nonlocal value` reassigns the enclosing binding; this program contains no generator. |
| `file-iter/10.err` → `10.out` | Empty successful output: a valid enclosing `nonlocal` declaration needs no refusal. |
| `syntax-modern/1.err` | Main's existing `with` lowering evaluates the header and encounters unavailable `open`. The old blanket context-manager refusal is obsolete. This is an integration expectation, not a claim of CPython context-manager conformance: `__enter__` and `__exit__` remain unimplemented. |
| `syntax-modern/2.err` | Main already evaluates tuple expressions, so `return *a, 5` succeeds and the program reaches the unsupported starred subscript. The preceding tuple and iteration output is retained. Tuple representation remains an existing limitation. |
| `syntax-modern/6.err` | Uses main's existing generator-refusal wording. Generator execution remains unsupported, and the body must not start. |
| `syntax-modern/7.err` | Uses main's existing unsupported-deletion wording. This does not claim support for slice deletion. |
| `syntax-modern/20.err` | Uses main's existing unsupported-class-form wording. The kernels are also fixed so this rejected decorated class does not evaluate its decorators, bases, or body. |
| `syntax-modern/26.err` | Uses the same integer-identity limitation diagnostic as `expressions/14`; the two programs test the same case. This does not claim that CPython lacks integer identity. |

The six integration/diagnostic changes above are separate from the CPython
closure corrections. They retain main's existing features and failure
categories instead of disabling features to restore obsolete refusals.

## Expectations preserved by kernel fixes

- `syntax-modern/3.out` and `13.out`: lambda positional-only, keyword-only,
  variadic positional and variadic keyword arguments use the call binder.
- `syntax-modern/10.out`: late binding still produces `7`; the nested lambda
  still produces `9`.
- `syntax-modern/15.out`: deleting and rebinding the captured name still
  produces `9`.
- `syntax-modern/23.err`: the long-string reader again reports the specific
  unterminated quote diagnostic.
- `file-scope/10.err`: a class's `nonlocal` declaration must not turn an
  enclosing function parameter into that function's own `nonlocal`.

## Added regressions

| Files | Purpose |
|---|---|
| `syntax-modern/29.py`, `29.out` | Captured collection reassignment and deletion leave aliases and definition-time defaults attached to their original collection; scalar defaults also retain their definition-time value. |
| `syntax-modern/30.py`, `30.out` | Arithmetic and chained comparisons load captured bindings correctly; nested collection mutation and deletion preserve aliases. |
| `syntax-modern/31.py`, `31.out` | Reading a class-level `nonlocal` in an untaken branch does not contaminate the enclosing function's declarations. |

## Fixtures brought in unchanged by the closures merge

The merge adds `closures/0.py` through `closures/14.py` and their existing
expected files without rewriting them:

| Source | Expected file | Coverage |
|---|---|---|
| `closures/0.py` | `0.out` | Enclosing local read. |
| `closures/1.py` | `1.out` | Persistent nonlocal counter. |
| `closures/2.py` | `2.out` | Enclosing parameter and late-bound comprehension variable. |
| `closures/3.py` | `3.err` | Local shadowing and an unbound local. |
| `closures/4.py` | `4.out` | Transitive capture, global assignment, later binding, and recursion. |
| `closures/5.py` | `5.out` | Shadowing, loop late binding, and definition-time defaults. |
| `closures/6.py` | `6.err` | Free variable read before its enclosing assignment. |
| `closures/7.py` | `7.out` | Deletion and reassignment through shared bindings. |
| `closures/8.py` | `8.err` | Unbound local arithmetic. |
| `closures/9.py` | `9.err` | Enclosing local read before assignment after capture. |
| `closures/10.py` | `10.out` | Capturing lambdas with defaults and parameter markers. |
| `closures/11.py` | `11.err` | Missing enclosing binding for `nonlocal`. |
| `closures/12.py` | `12.err` | Augmented assignment to an unbound local. |
| `closures/13.py` | `13.err` | A bare annotation declares a local. |
| `closures/14.py` | `14.out` | Separate comprehension invocations and assignment-expression capture. |

The merge also carries these existing expectation corrections:

- `blocks/6.err` → `6.out`: valid nonlocal assignment, empty output.
- `expressions/7.err` → `7.out`: enclosing parameter capture prints `3`.
- `file-scope/4.err` → `4.out`: valid nonlocal increment, empty output.

All builds and program execution are performed in the repository's CI.
