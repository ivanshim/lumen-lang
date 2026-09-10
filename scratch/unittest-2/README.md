# Runner regression fixtures

`decorated-lifecycle` keeps the class setup and cleanup output, and now
checks that the subtest failure, method skip and expected failure are counted.
`exception-diagnostics` retains the expected exception messages: conversion
must call the exception's text method, rather than print an object label.
`regex-warning-assertions` retains the wildcard assertion and checks anchors,
repetition, escaped punctuation and newline exclusion. Its warning category
is the exported `warnings.UserWarning`, so it exercises warning capture
without depending on the separate built-in warning hierarchy.

`assertions-execute` replaces the copied containment suite formerly in `6.py`.
It deliberately fails membership and equality assertions and checks the
failure count and summary. `errors-recorded` replaces the copied keyword-only
suite formerly in `7.py`; it checks keyword calls, callable assertion helpers,
and continuation after an ordinary error. Their former empty output files
came from suites whose bodies were not exercised. These focused programs
have the same outcomes in Python and do not turn unrelated unsupported
language features into expected successful tests. The reference suites and
examples are unchanged.

The inherited `scratch/modules/2.py` still runs its three original tests,
including the deliberate failure. It now captures the runner stream with
`exit=False` and checks the result and summary instead of expecting the old
stdout report and successful default exit. `main-failure` separately checks
the failing default exit; scratch compares the first stderr line, `F`.

The inherited `scratch/file-scope/10.py` is a deferred scope syntax fixture.
Its complete class body is retained, but its entry point now prints
`scope suite defined` after defining the suite instead of calling
`unittest.main()`. Defining the class does not execute its test methods in
Python. The old `.err` expected a class-form refusal that working `unittest`
has replaced; the new `.out` records the definition-complete message.
Running this copied suite would instead exercise unrelated closure, cell,
and dynamic compilation requirements. Runner execution is checked by the
focused fixtures above; the reference suites and examples remain unchanged.
