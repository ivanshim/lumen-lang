class Bad:
    def __eq__(self, other):
        raise ValueError()
x = slice(Bad())
print(x == x)
try:
    print(x == slice(Bad()))
except ValueError:
    print("comparison raised")
class Bound:
    def __index__(self):
        raise ValueError()
try:
    print([1, 2][Bound():])
except ValueError:
    print("bound raised")
class G:
    def __getitem__(self, key):
        raise ValueError()
try:
    print(G()[1:2])
except ValueError:
    print("key raised")
try:
    slice(Bound(), None).indices(3)
except ValueError:
    print("indices raised")
import operator
try:
    operator.itemgetter(slice(1, 2))(G())
except ValueError:
    print("getter raised")
