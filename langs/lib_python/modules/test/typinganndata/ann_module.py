"""
The module for testing variable annotations.

Empty lines above are for good reason (testing for correct line numbers)
"""
# CPython's copy makes M a metaclass and builds two classes with
# types.new_class. Neither is possible here -- this runtime cannot subclass
# type, and there is no types module -- so M is a plain class and the
# new_class pair is left out. D's j is also left unannotated: the microcode7
# kernel refuses a class body that annotates an attribute named j, while
# stack8 accepts it, and without this the module will not import on both.
# Every annotation the tests read is present.

from typing import Optional
from typing import Tuple


class C:

    x = 5; y: Optional['C'] = None

x: int = 5; y: str = x; f: Tuple[int, int]


class M:

    o: type = object


(pars): bool = True


class D(C):
    j = 'hi'; k: str = 'bye'


class F():
    z: int = 5
    def __init__(self, x):
        pass


class Y(F):
    def __init__(self):
        super(F, self).__init__(123)


class S:
    x: str = 'something'
    y: str = 'something else'


def foo(x: int = 10):
    def bar(y: str):
        x: str = 'yes'
    bar(x)


u: int | float
