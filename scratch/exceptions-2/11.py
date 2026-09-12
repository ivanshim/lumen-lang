try:
    assert False, ""
except AssertionError as e:
    print(len(e.args))
try:
    assert False
except AssertionError as e:
    print(len(e.args))
e = ValueError("x")
e.__cause__ = None
print(e.__suppress_context__)
e.__suppress_context__ = False
print(e.__suppress_context__)
