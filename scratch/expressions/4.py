def choose(x):
    return 1 if x else 2 if True else 3
print(choose(True), choose(False), 7 if True else absent())
print((lambda: 8)(), (lambda *a: len(a))(1, 2, 3))
x = 4
f = lambda a=x: a
x = 9
print(f(), (lambda x = lambda y = lambda z=1: z: y(): x())())
print((lambda a, /, b=3: a + b)(2))
def empty(): ...
print(empty(), ... is ..., ... == "Ellipsis")
