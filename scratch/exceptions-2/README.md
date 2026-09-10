# Exception continuation checks

These programs cover exception groups, notes, causes, traceback output,
exception metadata, and the boundary between generator exhaustion and a
stop-iteration exception raised inside the generator body. Quoted list
contents are checked through explicit `repr` calls, preserving the ordinary
list rendering measured by the examples on all six kernels.

Inherited fixture corrections:

- `scratch/exceptions/8.err` and `11.err` now expect the traceback header
  for uncaught assertions, replacing the earlier generic uncaught message.
- `scratch/exceptions/12.err` now expects the traceback header for raising
  a string, which raises a type error under Python's rules.
- `scratch/exceptions/13.err` now expects the traceback header for an
  invalid handler selector, which raises a type error.
- `scratch/exceptions-classes/3.err` now expects the traceback header for
  its uncaught type error.
- `scratch/exceptions/9.err` becomes `9.out`, containing `not reached`.
  Despite that text, the try body is reached and succeeds. No exception
  occurs, so Python does not evaluate the undefined `E` selector. The
  former refusal of exception groups no longer describes this program.

The `.err` fixtures compare only the first stderr line. The exception
continuation probes separately exercise class names, messages and notes.

Ordinary for loops now consume generators one item at a time, so output
from an earlier yield is retained when a later step raises. String methods
sharing an exception-method name still dispatch on the receiver type.
