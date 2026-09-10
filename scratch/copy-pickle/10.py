import copy
import pickle
x = [1]
t = (1, "a")
b = copy.copy(x)
print(b is x, b == x)
print(copy.copy(t) is t, copy.deepcopy(t) is t)
r = pickle.loads(pickle.dumps(t))
print(r == t, copy.copy(r) is r)
s = [x]
c = copy.copy(s)
print(c is s, c[0] is x)
