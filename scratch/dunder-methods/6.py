class Steps:
    def __iter__(self):
        print("iter")
        return self
    def __next__(self):
        print("next")
        return 7
for value in Steps():
    print("body", value)
    break
class C:
    def __mul__(self, other):
        return other + 1
    __rmul__ = __mul__
print(3 * C())
c = C()
d = c.__dict__
d["x"] = 4
print(c.x)
c.x = 5
print(d["x"], len(d))
del d["x"]
print(len(c.__dict__))
