def inner():
    yield 4
    return 9
def outer():
    result = yield from inner()
    yield result
print(list(outer()))
print(tuple(outer()))
def source():
    print("source starts")
    yield 1
    print("source resumes")
    yield 2
def square(x):
    print("square", x)
    return x * x
it = (square(x) for x in source())
print("made")
print(next(it))
print(next(it))
print(next(it, "done"))
