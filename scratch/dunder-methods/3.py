class A:
    def __init__(self, x):
        self.x = x
class B(A):
    def __init__(self, x):
        super().__init__(x + 1)
class C(B):
    def __init__(self, x):
        super().__init__(x + 1)
    def __getattr__(self, name):
        return "missing " + name
    def __enter__(self):
        print("enter")
        return self
    def __exit__(self, kind, value, trace):
        print("exit")
with C(1) as v:
    print("body", v.x)
print(v.unknown)
