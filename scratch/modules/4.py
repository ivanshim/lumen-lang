import sys, math, operator, functools, itertools, collections, copy, random, gc, weakref, warnings, unittest
from test import support
from test.support import import_helper, os_helper
from io import StringIO
print('modules loaded')
from math import *
import math as numbers
print(sqrt(9), numbers.factorial(5), gcd(12, 18))
gc.disable()
print(gc.isenabled(), gc.collect())
gc.enable()
print(gc.isenabled())
print(weakref.ref([1, 2])())
print(import_helper.import_module('math') is math)
print(StringIO('hello').getvalue(), support.verbose, os_helper.TESTFN)
