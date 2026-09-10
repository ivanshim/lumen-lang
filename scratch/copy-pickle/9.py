import pickle
x = [1]
y = [x, x]
z = pickle.loads(pickle.dumps(y))
print(z[0] is z[1], z[0] is x)
