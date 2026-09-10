# Inherited fixture corrections

The preceding run disagreed with eleven inherited expectations on both full
kernels. These corrections retain the examples and reference suites unchanged.
The rules used are Python's [compound statements](https://docs.python.org/3/reference/compound_stmts.html)
and [simple statements](https://docs.python.org/3/reference/simple_stmts.html).

| Fixture | Change and reason |
| --- | --- |
| `scratch/blocks/0` | Replace `.out` with `.err`: an integer is not a context manager, so `with 5` raises `TypeError` before binding or printing. |
| `scratch/blocks/7` | Keep the output; wrap each value in a real context manager. Replace the module-level `async with` with `with`: it was invalid outside a coroutine and the list was not an asynchronous context manager. The program still checks multiple managers, parenthesized managers, omitted targets and tuple binding. |
| `scratch/blocks/14` | Keep the output; wrap values in real context managers and use an ordinary `for` over the list. The former module-level `async for` was invalid, and a list is not an asynchronous iterator. Parenthesized and nested binding, iteration and loop `else` remain exercised. |
| `scratch/blocks/16` | Expect the context-manager `TypeError`, not a binding refusal: the list lacks the protocol, so entry fails before tuple unpacking. |
| `scratch/blocks/17` | Expect the context-manager `TypeError`, not a starred-binding refusal: the list lacks the protocol, so entry fails before binding. |
| `scratch/blocks/24` | Replace `.out` with `.err`: `with 5` fails the context-manager check before assigning the subscript or executing either body. |
| `scratch/exceptions/7` | Expect `RuntimeError: No active exception to reraise`: a bare raise outside an active handler is a runtime error. |
| `scratch/exceptions/8` | Expect `AssertionError: no`, without the old `PythonError: Uncaught` wrapper. |
| `scratch/exceptions/11` | Expect `AssertionError`, without the old `PythonError: Uncaught` wrapper. |
| `scratch/exceptions/12` | Expect `TypeError: exceptions must derive from BaseException`: raising a string is invalid; the final bare raise restores the outer handler's error after the inner handler ends. |
| `scratch/file-exceptions/28` | Expect the next unsupported name, `TestCase`, instead of `Exception`. The built-in exception base now exists, so the imported suite advances to `unittest.TestCase`. This remains an implementation limitation, not a claim that the suite passes under Python. |

The parameters refusals had already been replaced before this resumption:
`scratch/params/15.err` became `15.out` containing `1` without a newline
(`end=""` is valid), and `scratch/params/16.err` became `16.out` containing
`[]` and a newline (a list default is valid). Those fixtures need no further
change and are left as inherited.

The numbered programs in this directory cover cleanup on return, overriding
returns in `finally`, exception context and explicit causes, context-manager
exit arguments and unwinding, and loop completion versus early exit.
