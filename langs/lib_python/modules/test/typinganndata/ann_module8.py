# From CPython v3.14.8 (8e6e75d9102e), Lib/test/typinganndata/ann_module8.py; PSF License.
# Test `@no_type_check`,
# see https://bugs.python.org/issue46571

class NoTypeCheck_Outer:
    class Inner:
        x: int


def NoTypeCheck_function(arg: int) -> int:
    ...
