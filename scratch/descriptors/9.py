class P(property):
    def __get__(self, obj, owner):
        print('get')
        return super().__get__(obj, owner)
print('P')
class C:
    def value(self): return 7
    x = P(value)
print('C')
print(C().x)
def f(): return 1
f.__dict__['author'] = 'Cleese'
print(f.author, f.__dict__['author'])
