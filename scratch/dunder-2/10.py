class D(dict):
    def __missing__(self, key):
        return "missing"
d = D({"one": 1})
d["two"] = 2
print(d["one"], d["two"], d["absent"])
print(d.__dict__)
