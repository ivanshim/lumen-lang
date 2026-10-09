# Keep the upstream modules' package names when loading their own test hooks.
from test.test_doctest import test_doctest, test_doctest2
import unittest


def load_tests(loader, tests, pattern):
    for module in (test_doctest, test_doctest2):
        tests.addTests(loader.loadTestsFromModule(module))
    return tests


def _cases(suite):
    for case in suite:
        if isinstance(case, unittest.TestSuite):
            yield from _cases(case)
        else:
            yield case


def _matches(case, name):
    identifier = case.id()
    return (identifier == name or identifier.startswith(name + '.')
            or identifier.endswith('.' + name) or '.' + name + '.' in identifier)


if __name__ == '__main__':
    import sys
    names = sys.argv[1:]
    if names and all(not name.startswith('-') for name in names):
        # Doctest IDs identify discovered cases, not callable module attributes.
        cases = list(_cases(load_tests(unittest.defaultTestLoader,
                                       unittest.TestSuite(), None)))
        for name in names:
            if not any(_matches(case, name) for case in cases):
                raise ValueError('No discovered test matches ' + repr(name))
        suite = unittest.TestSuite(case for case in cases
                                   if any(_matches(case, name) for name in names))
        result = unittest.TextTestRunner().run(suite)
        sys.exit(not result.wasSuccessful())
    else:
        unittest.main()
