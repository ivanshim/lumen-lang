class WideHash:
    def __hash__(self):
        return 2305843009213693951
print(hash(WideHash()))
class Base:
    def __eq__(self, other):
        print("base")
        return NotImplemented
class Derived(Base):
    def __eq__(self, other):
        print("derived")
        return NotImplemented
print(Base() == Derived())
class Nope:
    def __bool__(self):
        return 1
try:
    bool(Nope())
except TypeError as e:
    print(e)
