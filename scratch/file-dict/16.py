a = {1: 2, 3: 4}
b = {3: 4, 1: 2}
print(a == b)
print(a != b)
print(a == {1: 2, 3: 5})
print(a == {1: 2})
print({"outer": [a]} == {"outer": [b]})
print([a, 1] == [b, 1])
print([a, 1] == [1, b])
print({1: {2: 3, 4: 5}} == {1: {4: 5, 2: 3}})
print(list(a), list(b))
