try:
    raise ValueError
except ValueError as e:
    print(len(e.args))
try:
    try:
        raise ValueError("v")
    except 3:
        print("wrong")
except TypeError as e:
    print(e)
try:
    raise KeyboardInterrupt
except Exception:
    print("wrong")
except BaseException:
    print("interrupt")
try:
    assert False, "assertion message"
except AssertionError as e:
    print(str(e), e.args[0])
print(NameError("n", name="lost").name)
obj = ValueError()
e = AttributeError("a", name="missing", obj=obj)
print(e.name, e.obj is obj)
def recur():
    return recur()
try:
    recur()
except RecursionError:
    print("recursion")
