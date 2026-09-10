import pickle
v = {"k": [1, 2.5, (3, None)], "s": {1, 2}}
print(pickle.loads(pickle.dumps(v)) == v, pickle.loads(pickle.dumps(2**100)), pickle.loads(pickle.dumps("é")))
