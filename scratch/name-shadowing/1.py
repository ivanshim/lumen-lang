len = len

def f(): return len([1, 2, 3])
print(f())
len = lambda a: 13
print(f())
del len
print(f())

super = super
super = lambda a: a + 1
print(super(4))

def params(*len): return len[0](7)
def kwargs(**len): return len['f'](8)
print(params(lambda a: a + 2), kwargs(f=lambda a: a + 3))

def factories():
    object = lambda a: a + 1
    type = lambda a: a + 2
    str = lambda a: a + 3
    bytes = lambda a: a + 4
    bytearray = lambda a: a + 5
    range = lambda a: a + 6
    print(object(1), type(1), str(1), bytes(1), bytearray(1), range(1))
factories()

def round_reader(): return round(2.0)
print(round_reader())
round = lambda a: 25
print(round_reader())
del round
print(round_reader())
