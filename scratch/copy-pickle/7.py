import pickle
import io
f = io.StringIO()
pickle.dump({"x": 2**100}, f, protocol=0)
f.seek(0)
print(pickle.load(f)["x"])
g = io.StringIO()
pickle.Pickler(g, 5).dump("é")
g.seek(0)
print(pickle.Unpickler(g).load())
