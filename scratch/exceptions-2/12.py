# Exhaustion is a catchable class with no message; escaping it is a runtime fault.
it = iter([])
try:
    next(it)
except StopIteration as e:
    print(type(e).__name__, len(e.args), repr(str(e)))
g = ExceptionGroup("tuple", (ValueError("v"), TypeError("t")))
print(g.message, len(g.exceptions))
a, b = g.split(ValueError)
print(type(a.exceptions[0]).__name__, type(b.exceptions[0]).__name__)
