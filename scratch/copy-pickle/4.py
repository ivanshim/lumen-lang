import pickle
for v in [None, True, False, 2**100, -2**100, 2.5, -0.0, "é", "a\0b", {"x": 3}]:
    print(pickle.loads(pickle.dumps(v)) == v)
