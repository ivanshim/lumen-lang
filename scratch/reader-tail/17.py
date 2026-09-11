# test_decorators.py:290, a call upon a member call answer.
class C:
    def make(self, x):
        return x
def identity(x):
    return x
c = C()
print(c.make(identity)(c.make(identity)(7)))
