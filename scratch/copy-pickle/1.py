import copy
x = [1]
y = [x, x]
z = copy.deepcopy(y)
print(z[0] is z[1], z[0] is x)
s = []
s.append(s)
t = copy.deepcopy(s)
print(t[0] is t)
