def value(me, x):
    return x + 1

class C:
    f = value
    def get(me, x):
        return x + 2
    again = get
    class Inner: pass
    made = Inner()

c = C()
print(c.f(3), c.again(3))
def replace():
    C.get = value
    return 3

print(c.get(replace()))
print(c.get(3))
