g = ExceptionGroup("g", [ValueError(1), TypeError(2)])
print(g.message, len(g.exceptions))
print(type(g.exceptions[0]).__name__, type(g.exceptions[1]).__name__)
try:
    raise g
except* ValueError as e:
    print("V", len(e.exceptions))
except* TypeError as e:
    print("T", len(e.exceptions))
print(str(g))
print(str(OSError(2, "missing")))
