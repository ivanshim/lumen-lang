import unittest
from test.support import captured_stdout, captured_stderr
import sys
@unittest.skipUnless(False, 'not enabled')
def skipped():
    print('should not run')
try:
    skipped()
except unittest.SkipTest:
    print('skipped')
print(skipped.__unittest_skip_why__)
with captured_stdout() as outer:
    print('outer')
    with captured_stderr() as errors:
        print('error', file=sys.stderr)
        with captured_stdout() as inner:
            print('inner')
    print('after')
print(outer.getvalue(), end='')
print(inner.getvalue(), end='')
print(errors.getvalue(), end='')
