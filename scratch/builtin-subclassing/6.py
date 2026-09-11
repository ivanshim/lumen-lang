class S(str):
    __slots__ = ("note",)
s = S("ab")
s.note = 2
print(s, s.note, hasattr(s, "__dict__"))
try:
    s.extra = 3
except AttributeError:
    print("blocked")
