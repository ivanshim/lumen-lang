class C:
    __slots__ = ("x",)
    def __new__(cls):
        print("new")
        return object.__new__(cls)
    def __init__(self):
        print("init")
        self.x = 1
obj = C()
try:
    obj.z = 1
except AttributeError as e:
    print("AttributeError:", str(e))
class Counter:
    count = 0
    def __setattr__(self, name, value):
        Counter.count += 1
        object.__setattr__(self, name, value)
c = Counter()
c.x = 1
c.x = 2
print(Counter.count, c.x)
