class Counter:
    def __init__(self):
        self.i = 0
    def __iter__(self):
        return self
    def __next__(self):
        if self.i == 3:
            raise StopIteration
        self.i = self.i + 1
        return self.i
print(list(Counter()))
for i in Counter():
    print(i)
c = Counter()
print(next(c), next(c))
class Left:
    def __add__(self, other):
        return NotImplemented
class Right:
    def __radd__(self, other):
        return 42
print(Left() + Right())
class Context:
    def __enter__(self):
        print("enter")
        return self
    def __exit__(self, kind, value, trace):
        print("exit", kind is None)
        return True
with Context():
    raise StopIteration()
print("suppressed")
