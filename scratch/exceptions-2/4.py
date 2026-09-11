import sys
print(sys.exception() is None, sys.exc_info()[2] is None)
e = ValueError("x")
e.add_note("first")
e.add_note("second")
print(len(e.__notes__), e.__notes__[1], e.__traceback__ is None)
print(e.__suppress_context__)
try:
    raise e from None
except ValueError as caught:
    print(caught.__suppress_context__, sys.exception() is caught)
    print(sys.exc_info()[0].__name__, sys.exc_info()[1] is caught)
print(sys.exception() is None)
g = ExceptionGroup("outer", [ValueError("v"), ExceptionGroup("inner", [TypeError("t")])])
a, b = g.split(ValueError)
print(a.message, len(a.exceptions), b.exceptions[0].message)
print(g.subgroup(KeyError) is None)
print(g.derive([KeyError("k")]).message)
try:
    try:
        raise g
    except* ValueError:
        print("value")
except ExceptionGroup as remainder:
    print(remainder.message, remainder.exceptions[0].message)
try:
    raise 3
except TypeError as caught:
    print(caught)
try:
    missing_exception_name
except NameError as caught:
    print(caught.name)
try:
    e.absent
except AttributeError as caught:
    print(caught.name, caught.obj is e)
print(OSError(2, "missing").errno, OSError(2, "missing").strerror)
