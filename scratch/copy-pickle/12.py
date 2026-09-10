import pickle
import io
f = io.StringIO()
pickle.dump(1, f)
pickle.dump(2, f)
f.seek(0)
print(pickle.load(f), pickle.load(f))
