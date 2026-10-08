# Preserve standalone suite loading while keeping the upstream library unchanged.
import _doctest_compat


class _SuiteFinder:
    """Supply the caller module to the legacy suite's finder interface."""

    def __init__(self, module, finder):
        self.module = module
        self.finder = DocTestFinder() if finder is None else finder

    def find(self, obj, *args, **kwargs):
        return self.finder.find(self.module if obj is None else obj, *args, **kwargs)


# Preserve the suite's established main-module namespace without changing imported modules.
def _adapt_suite(upstream):
    @functools.wraps(upstream)
    def suite(module=None, globs=None, extraglobs=None, test_finder=None, **options):
        module = _normalize_module(module)
        if module.__name__ == '__main__' and globs is None:
            cases = _doctest_compat.DocTestSuite(
                globs=globs, extraglobs=extraglobs,
                test_finder=_SuiteFinder(module, test_finder), **options)
            return _DocTestSuite(cases)
        return upstream(module, globs, extraglobs, test_finder, **options)
    return suite


DocTestSuite = _adapt_suite(DocTestSuite)
del _adapt_suite
