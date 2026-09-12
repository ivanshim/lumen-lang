d = {"b": 1, "a": 2}
e = {"a": 2.0, "b": 1.0}
print(d == e, d != e, len(d))
for k in d:
    print(k)
u = d | {"b": 4, "c": 3}
for k in u:
    print(k, u[k])
x = {True: "first", 1: "last"}
print(len(x), x[1], x[True])
x[1.0] = "new"
print(len(x), x[True])
print({"a": {"x": 1, "y": 2}} == {"a": {"y": 2.0, "x": 1}})
