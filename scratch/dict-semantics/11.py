d = {"a": 1}
a = d
d |= {"a": 2, "b": 3}
print(d, a, d is a)
d |= [("c", 4)]
print(a)
