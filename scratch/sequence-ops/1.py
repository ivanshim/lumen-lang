t = (1, [2])
t[1].append(3)
print(t)
try:
    t[0] = 9
except TypeError as e:
    print(str(e))
print(hash((1, 2)) == hash((1, 2)))
