import math

class FloatsAreIdenticalMixin:
    def assertFloatsAreIdentical(self, first, second, msg=None):
        if math.isnan(first):
            self.assertTrue(math.isnan(second), msg)
        else:
            self.assertEqual(first, second, msg)
            if first == 0:
                self.assertEqual(math.copysign(1.0, first), math.copysign(1.0, second), msg)

class ComplexesAreIdenticalMixin:
    def assertComplexesAreIdentical(self, first, second, msg=None):
        raise 'NotImplementedError: complex identity checks need complex number support'
