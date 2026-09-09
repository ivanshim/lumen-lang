def declarations():
    class A:
        def value(self):
            return 1
    class B(): pass
    class C(A, B):
        class D: pass
print('read')
