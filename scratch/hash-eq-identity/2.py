class C:
    def __eq__(self, other):
        return True
try:
    hash(C())
except TypeError as e:
    print(e)
class H:
    def __eq__(self, other):
        return isinstance(other, H)
    def __hash__(self):
        return 7
h = H()
d = {h: "found"}
print(d[H()], hash(h), bool(object()), True + True)
