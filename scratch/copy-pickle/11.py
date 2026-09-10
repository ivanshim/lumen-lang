import copy
class Item:
    def __deepcopy__(self, memo):
        return 9
x = Item()
print(copy.deepcopy([x, x]))
class Link:
    pass
x = Link()
x.next = x
y = copy.deepcopy(x)
print(y.next is y, y is x)
