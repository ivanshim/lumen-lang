""">>> x
42
"""
import doctest
import sys
import unittest

__test__ = {'sample': '>>> x\n42\n'}

# Keep discovery in the caller even when example globals use another name.
def example_member():
    """
    >>> x
    42
    """

# Collect and run all three kinds of docstrings with independent globals.
def check(label, **options):
    supplied = options.get('globs')
    suite = doctest.DocTestSuite(**options)
    assert suite.countTestCases() == 3
    print(label, suite.countTestCases())
    result = unittest.TestResult()
    suite.run(result)
    assert result.testsRun == 3 and not result.failures and not result.errors
    print(result.testsRun, len(result.failures), len(result.errors))
    if supplied is not None:
        print(sorted(supplied))

check('omitted', globs={'x': 42})
check('renamed', globs={'x': 42, '__name__': 'another_module'})
check('explicit', module=sys.modules[__name__], globs={'x': 42})
check('extra', globs={'x': 0}, extraglobs={'x': 42})
