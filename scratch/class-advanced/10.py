class A:
    def __init_subclass__(cls):
        print("registered", cls.__name__)
        super().__init_subclass__()
class B(A):
    def __setattr__(self, name, value):
        super().__setattr__(name, value)
b = B()
b.x = 4
print(b.x)
class Outer:
    class Inner:
        class Leaf:
            def f(self): pass
print(Outer.Inner.Leaf.__qualname__, Outer.Inner.Leaf.f.__qualname__)
