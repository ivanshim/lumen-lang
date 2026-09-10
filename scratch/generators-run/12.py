def source():
    print("outer source")
    return range(2)
def inner(x):
    print("inner", x)
    return range(2)
it = (x * 10 + y for x in source() for y in inner(x) if y > 0)
print("made")
print(list(it))
factor = 2
it = (factor * x for x in range(3))
factor = 5
print(list(it))
print(list((x, x * x) for x in range(3)))
def never():
    print("must not run")
    yield 1
it = never()
print(it.close())
print(next(it, 42))
