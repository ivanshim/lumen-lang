class Member:
    def __set_name__(self, owner, name):
        print(owner.__name__, name)
class Base:
    def __init_subclass__(cls, flag=0):
        print(cls.__name__, flag)
class Child(Base, flag=4):
    field = Member()
    def __class_getitem__(cls, key):
        return key + 1
print(Child[2])
class Text:
    def __str__(self):
        return "text"
print(format(Text(), ""), object.__format__(Text(), ""))
