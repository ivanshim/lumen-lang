def h(d={}):
    d["k"] = len(d) + 1
    return d
print(h()["k"])
print(h()["k"])
