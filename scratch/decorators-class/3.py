def outer(f):
    print('outer')
    return f
def inner(f):
    print('inner')
    return f
def choose(label):
    print(label)
    return inner
class C:
    @outer
    @choose(
        'choose',
    )
    def m(self):
        return 4
print(C().m())
