d = {"a": [1, {"b": None}]}
print(d, len(d), len(d["a"]), len(d["a"][1]))
def collect(**kw):
    return kw
print(collect(**dict(a=1, b=2)))
print({k: v for k, v in [(1, 2), (3, 4)] if k > 1})
print({k: {v: k + v for v in range(2)} for k in range(2)})
