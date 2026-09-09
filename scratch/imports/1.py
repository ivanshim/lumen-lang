import a
import a.b.c
import a as b
import a, b
from a import b
from a import b as c, d
from a import (b,
              c)
from a import *
from . import x
from .a import b
from ..a import b
print(a, b, c, x)
