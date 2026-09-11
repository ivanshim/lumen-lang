try:
    raise 3
except TypeError as e:
    print(e)
try:
    raise SystemExit(3)
except Exception:
    print("wrong")
except BaseException:
    print("BaseException")
def g():
    yield 1
    raise StopIteration
try:
    for x in g():
        print(x)
except RuntimeError as e:
    print(e)
