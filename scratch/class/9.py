class D:
    def append(me, a, b):
        return a + b

print(D().append(2, 5))
class W:
    append = D().append

print(W().append(4, 5))
objects = [D()]
print(objects[0].append(6, 7))
