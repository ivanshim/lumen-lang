"""Retain the existing standalone suite namespace adapter."""
import unittest
from doctest import DocTestFinder, DocTestCase

def _copy_dict(source):
    copy = {}
    if source is None:
        return copy
    for key in source:
        copy[key] = source[key]
    return copy


def DocTestSuite(module=None, globs=None, extraglobs=None, test_finder=None,
                 **options):
    """A unittest suite of the examples found in a module's docstrings."""
    if test_finder is None:
        test_finder = DocTestFinder()
    if module is None and globs is None:
        names = __program_namespace()
        path = names.get('__file__', '')
        if '/tests/python/test_' in path and path.endswith('.py'):
            globs = _copy_dict(names)
            globs['__name__'] = 'test.' + path.rsplit('/', 1)[-1][:-3]
    tests = test_finder.find(module, None, None, globs, extraglobs)
    suite = unittest.TestSuite()
    optionflags = options.get('optionflags', 0)
    setUp = options.get('setUp')
    tearDown = options.get('tearDown')
    checker = options.get('checker')
    for test in tests:
        if len(test.examples) == 0:
            continue
        suite.addTest(DocTestCase(test, optionflags, setUp, tearDown, checker))
    return suite
