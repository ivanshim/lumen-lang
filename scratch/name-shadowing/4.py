import os
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, '.')
with open('name_shadowing_star_fixture.py', 'w') as f:
    f.write('def len(a, b): return a + b\ndef sum(a, b): return a + b\ndef bytes(a): return a + 10\n')
with open('name_shadowing_star_consumer.py', 'w') as f:
    f.write('def reader(): return len(4, 5)\nfrom name_shadowing_star_fixture import *\nanswer = len(2, 3)\n')
print('before', len([1, 2]))
def reader(): return len(1, 2)
from name_shadowing_star_fixture import *
print('after', len(1, 2), reader(), sum(3, 4))
print('fallback', abs(-3), round(2.0), list(range(3)))
saved = bytes
print('byte name', bytes(5), saved(6), type(b'x') is bytes)
class Base:
    def f(self): return 8
class Child(Base):
    def f(self): return super().f()
print('parent', Child().f())
import name_shadowing_star_consumer as consumer
print('module', consumer.answer, consumer.reader(), consumer.len(7, 8))
with open('name_shadowing_star_noncallable.py', 'w') as f:
    f.write('len = None\n')
from name_shadowing_star_noncallable import *
try:
    len([])
except TypeError as error:
    print('noncallable', type(error).__name__)
os.remove('name_shadowing_star_noncallable.py')
del len
print('deleted', len([1, 2, 3]))
os.remove('name_shadowing_star_fixture.py')
os.remove('name_shadowing_star_consumer.py')
