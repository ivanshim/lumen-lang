for kind in [BaseException, Exception, ArithmeticError, ZeroDivisionError, OverflowError, LookupError, IndexError, KeyError, TypeError, ValueError, NameError, UnboundLocalError, AttributeError, RuntimeError, NotImplementedError, StopIteration, AssertionError, SystemExit, KeyboardInterrupt, ImportError, OSError, RecursionError, UnicodeError]:
    e = kind("x")
    print(type(e).__name__, repr(e), e.args)
print(repr(ValueError()), repr(str(ValueError())))
print(repr(ValueError("a", "b")), str(ValueError("a", "b")))
try:
    raise ZeroDivisionError
except ArithmeticError as e:
    print("arithmetic", e.args)
try:
    raise UnboundLocalError
except NameError:
    print("name")
try:
    raise UnicodeError
except ValueError:
    print("value")
