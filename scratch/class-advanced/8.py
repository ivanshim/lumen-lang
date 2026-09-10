class C:
    def __new__(cls):
        print("allocating", cls.__name__)
        return super().__new__(cls)
    def __init__(self):
        print("initializing")
    def __call__(self, x):
        return x + 1
    def __getattr__(self, name):
        return name
c = C()
print(callable(c), c(4), c.missing, callable("len"), callable(len))
raw = object.__new__(C)
print(type(raw) is C, C.__mro__[-1] is object)
class Other: pass
class Factory:
    def __new__(cls): return Other()
    def __init__(self): print("must not run")
print(type(Factory()) is Other)
