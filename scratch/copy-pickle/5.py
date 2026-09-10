import copy
class Item:
    def __init__(self, value):
        self.value = value
    def __copy__(self):
        return Item(self.value + 1)
    def __deepcopy__(self, memo):
        return Item(self.value + 2)
a = Item(5)
print(copy.copy(a).value, copy.deepcopy(a).value)
s = "hello"
print(copy.copy(s) is s, copy.deepcopy(s) is s)
