a = [1]
b = a
before = id(a)
a.append(2)
print(a is b, id(a) == before, id(a) == id(b))
x = float("nan")
print(x is x, x is float("nan"), (x,) == (x,), {1: [x]} == {True: [x]})
print(1 == True, 0 == False, 1.0 == 1, "1" == 1, [] == (), None == None)
print(True is 1, isinstance(True, int), type(True) is bool, True + True)
print(hash(""), hash(()), hash((1, 2)) == hash((True, 2.0)), hash(None) == hash(None))
print(x < 1, x <= 1, x > 1, x >= 1)
