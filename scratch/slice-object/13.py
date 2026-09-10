class Bad:
    def __eq__(self, other):
        raise ValueError("comparison")
x = slice(Bad())
print(x == x)
try:
    print(x == slice(Bad()))
except ValueError:
    print("comparison raised")
class Bound:
    def __index__(self):
        raise ValueError("bound")
try:
    print([1, 2][Bound():])
except ValueError:
    print("bound raised")
class G:
    def __getitem__(self, key):
        raise ValueError("key")
try:
    print(G()[1:2])
except ValueError:
    print("key raised")
