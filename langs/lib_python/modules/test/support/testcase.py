# Assertions shared by tests of floating and complex numbers.
from math import copysign, isnan

class FloatsAreIdenticalMixin:
    def assertFloatsAreIdentical(self, x, y):
        if isnan(x) and isnan(y):
            return
        if x == y:
            if x != 0 or copysign(1.0, x) == copysign(1.0, y):
                return
        self.fail('floats are not identical')

class ComplexesAreIdenticalMixin(FloatsAreIdenticalMixin):
    def assertComplexesAreIdentical(self, x, y):
        self.assertFloatsAreIdentical(x.real, y.real)
        self.assertFloatsAreIdentical(x.imag, y.imag)
