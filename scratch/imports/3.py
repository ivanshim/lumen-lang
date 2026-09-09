import sys2
print(sys2)
import sys
print(sys)
import sys.stdout.write as writer
print(writer)
a = 7
def local_imports():
    import a.b.c
    from ...a import (b as c, d,)
    print(a, c, d)
local_imports()
print(a)
from a import *
print(a)
__name__ = "changed"
print(__name__)
