it = iter([4])
print(next(it))
try:
    next(it)
except StopIteration as e:
    print(repr(e), e.args)
print(next(it, "end"))
try:
    1 - "x"
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
e = ValueError("old")
e.args = ["new"]
print(e.args, str(e), repr(e))
try:
    {}[7]
except KeyError as e:
    print(e.args, str(e), repr(e))
try:
    assert False, "message"
except AssertionError as e:
    print(e.args, str(e), repr(e), e.__cause__)
