# Keep the upstream modules' package names when loading their own test hooks.
from test.test_doctest import test_doctest, test_doctest2
import unittest


def load_tests(loader, tests, pattern):
    for module in (test_doctest, test_doctest2):
        tests.addTests(loader.loadTestsFromModule(module))
    return tests


if __name__ == '__main__':
    unittest.main()
