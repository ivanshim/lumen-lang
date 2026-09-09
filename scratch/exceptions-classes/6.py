it = iter([4])
print(next(it))
try:
    next(it)
except StopIteration as e:
    print(repr(e), e.args)
print(next(it, "end"))
try:
    1 + "x"
except TypeError as e:
    print(type(e).__name__)
try:
    ValueError().missing
except AttributeError as e:
    print(type(e).__name__)
try:
    assert False, "no"
except Exception as e:
    print(type(e).__name__)
