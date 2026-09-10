import pickle
class Item:
    def __init__(self, number):
        self.number = number
    def __reduce__(self):
        return Item, (self.number + 1,)
print(pickle.loads(pickle.dumps(Item(8))).number)
def square(x):
    return x * x
print(pickle.loads(pickle.dumps(square))(7))
try:
    pickle.dumps(lambda: 0)
except pickle.PicklingError:
    print("lambda refused")
try:
    pickle.loads("broken")
except pickle.UnpicklingError:
    print("bad data refused")
