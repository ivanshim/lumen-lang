def first(self):
    "first doc"
    return 1
def second(self):
    "second doc"
    return 2
p = property(first, doc='kept')
q = p.getter(second)
print(p is q, q.__doc__, q.fget is second)
auto = property(first).getter(second)
print(auto.__doc__)
print(staticmethod(first).__get__(None) is first)
print(first.__get__(3)())
