class L(list):
    pass
class S(str):
    pass
class D(dict):
    pass
class T(tuple):
    pass
class U(set):
    pass
class I(int):
    pass
class F(float):
    pass
x = L([1])
alias = x
x += [2]
print(x, type(x).__name__, alias is x, len(alias))
x *= 2
print(x, type(x).__name__)
x += (7, 8)
print(x)
print(x + [9], type(x + [9]).__name__)
x.extend([0])
print(x, type(x).__name__)
y = S("a")
y += "b"
print(y, type(y).__name__)
y *= 2
print(y, type(y).__name__)
d = D({"a": 1})
kept = d
d |= {"b": 2}
print(d, type(d).__name__, kept is d)
d |= D({"c": 3})
print(d, type(d).__name__)
u = U({1, 2})
u |= {3}
u -= {1}
u &= {2, 3}
u ^= {3, 4}
print(sorted(u), type(u).__name__)
u |= U({8})
print(sorted(u), type(u).__name__)
t = T((1,))
t += (2,)
print(t, type(t).__name__)
t *= 2
print(t, type(t).__name__)
i = I(1)
i += 2
print(i, type(i).__name__)
i |= 4
i &= 6
i ^= 1
print(i, type(i).__name__, ~I(5))
f = F(1.5)
f += 1
print(f, type(f).__name__)
class Own(list):
    def __iadd__(self, other):
        return "own"
o = Own([1])
o += [2]
print(o, type(o).__name__)
class Adds(list):
    def __add__(self, other):
        return "added"
a = Adds([1])
a += [2]
print(a, type(a).__name__)
try:
    b = L([1])
    b += 5
except TypeError as e:
    print("TypeError", e)
