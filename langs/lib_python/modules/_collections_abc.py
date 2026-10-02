# _check_methods from CPython's Lib/_collections_abc.py at commit
# 3b564385e4c9 (the suite's commit), copied unchanged, under the PSF
# licence (tests/python/LICENSE). The container ABCs that share the
# reference's file live in collections/abc.py here; only the helper
# contextlib's structural hooks ask after is taken.

def _check_methods(C, *methods):
    mro = C.__mro__
    for method in methods:
        for B in mro:
            if method in B.__dict__:
                if B.__dict__[method] is None:
                    return NotImplemented
                break
        else:
            return NotImplemented
    return True
