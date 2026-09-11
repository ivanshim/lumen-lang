t = (1, [2])
t[1][0] = 9
print(t)
t[1][0] += 1
print(t)
try:
    t[1] += [3]
except TypeError as e:
    print(str(e))
print(t)
del t[1][0]
print(t)
