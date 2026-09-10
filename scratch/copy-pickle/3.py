import pickle
class Parcel:
    def __init__(self, value):
        self.value = value
    def __getstate__(self):
        return self.value + 1
    def __setstate__(self, state):
        self.value = state - 1
print(pickle.loads(pickle.dumps(Parcel(7))).value)
pickle.dumps(lambda: 0)
