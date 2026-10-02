# Assertions shared by tests of floating and complex numbers.
from math import copysign, isnan

# ExceptionIsLikeMixin is CPython's own, from Lib/test/support/testcase.py
# at the suite's commit 3b564385e4c9 under the PSF licence, copied unchanged.
class ExceptionIsLikeMixin:
    def assertExceptionIsLike(self, exc, template):
        """
        Passes when the provided `exc` matches the structure of `template`.
        Individual exceptions don't have to be the same objects or even pass
        an equality test: they only need to be the same type and contain equal
        `exc_obj.args`.
        """
        if exc is None and template is None:
            return

        if template is None:
            self.fail(f"unexpected exception: {exc}")

        if exc is None:
            self.fail(f"expected an exception like {template!r}, got None")

        if not isinstance(exc, ExceptionGroup):
            self.assertEqual(exc.__class__, template.__class__)
            self.assertEqual(exc.args[0], template.args[0])
        else:
            self.assertEqual(exc.message, template.message)
            self.assertEqual(len(exc.exceptions), len(template.exceptions))
            for e, t in zip(exc.exceptions, template.exceptions):
                self.assertExceptionIsLike(e, t)


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
