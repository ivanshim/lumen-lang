class C:
    def f(self):
        return 1
f = C.f
print(id(f) == id(C.f))
