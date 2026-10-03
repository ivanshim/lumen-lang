class Base:
    def value(self):
        return 7
class Child(Base):
    def value(self):
        return super(Child, self).value() + 1
    def empty(self):
        'a method docstring'
print(Child().value(), Child().empty())
row = ()
row += 1, 2
print(row)
for source in ('(a, b) += 1, 2', 'a, b += 1, 2'):
    try:
        compile(source, '<probe>', 'exec')
    except SyntaxError as error:
        print(type(error).__name__, error.msg)
a = bytearray(b'ab')
def obtain():
    print('owner')
    return memoryview(a)
obtain().cast('B')[:] = b'CD'
print(a)
