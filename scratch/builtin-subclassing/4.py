class S(str):
    label = "text"
    def __new__(cls, value):
        return str.__new__(cls, value)
    def __add__(self, other):
        return S(str(self) + str(other))
s = S("ab")
s.note = 3
print(s[0], "b" in s, list(s), s == "ab", hash(s) == hash("ab"))
print(type(s) is S, isinstance(s, S), isinstance(s, str))
print(type(s.upper()) is str, type(s + "c") is S, s + "c")
print(s.label, s.note, s.__dict__)
class A:
    label = "mapping"
class D(A, dict):
    pass
d = D(k=1)
print(d, d.label, isinstance(d, dict), isinstance(d, A))
