# The three pieces test_cmath borrows from the math test: where the complex
# test vectors live, the reader for that file's format, and the isclose test
# class it subclasses. The rest of the math test is a test file, not a
# library, and stays in tests/python.
#
# The vector file itself is not carried here, so opening test_file raises
# OSError. The name still says where CPython keeps it, beside the module in
# mathdata, which is what a caller that has the file would look for.
import math
import os
import unittest

NAN = float('nan')
NNAN = float('-nan')
INF = float('inf')
NINF = float('-inf')

test_dir = os.curdir
math_testcases = os.path.join(test_dir, 'mathdata', 'math_testcases.txt')
test_file = os.path.join(test_dir, 'mathdata', 'cmath_testcases.txt')


def parse_testfile(fname):
    """Parse a file with test values

    Empty lines or lines starting with -- are ignored
    yields id, fn, arg_real, arg_imag, exp_real, exp_imag
    """
    with open(fname, encoding="utf-8") as fp:
        for line in fp:
            # skip comment lines and blank lines
            if line.startswith('--') or not line.strip():
                continue

            lhs, rhs = line.split('->')
            id, fn, arg_real, arg_imag = lhs.split()
            rhs_pieces = rhs.split()
            exp_real, exp_imag = rhs_pieces[0], rhs_pieces[1]
            flags = rhs_pieces[2:]

            yield (id, fn,
                   float(arg_real), float(arg_imag),
                   float(exp_real), float(exp_imag),
                   flags)


class IsCloseTests(unittest.TestCase):
    isclose = math.isclose  # subclasses should override this

    def assertIsClose(self, a, b, *args, **kwargs):
        self.assertTrue(self.isclose(a, b, *args, **kwargs),
                        msg="%s and %s should be close!" % (a, b))

    def assertIsNotClose(self, a, b, *args, **kwargs):
        self.assertFalse(self.isclose(a, b, *args, **kwargs),
                         msg="%s and %s should not be close!" % (a, b))

    def assertAllClose(self, examples, *args, **kwargs):
        for a, b in examples:
            self.assertIsClose(a, b, *args, **kwargs)

    def assertAllNotClose(self, examples, *args, **kwargs):
        for a, b in examples:
            self.assertIsNotClose(a, b, *args, **kwargs)

    def test_negative_tolerances(self):
        # ValueError should be raised if either tolerance is less than zero
        with self.assertRaises(ValueError):
            self.assertIsClose(1, 1, rel_tol=-1e-100)
        with self.assertRaises(ValueError):
            self.assertIsClose(1, 1, rel_tol=1e-100, abs_tol=-1e10)

    def test_identical(self):
        # identical values must test as close
        identical_examples = [(2.0, 2.0),
                              (0.1e200, 0.1e200),
                              (1.123e-300, 1.123e-300),
                              (12345, 12345.0),
                              (0.0, -0.0),
                              (345678, 345678)]
        self.assertAllClose(identical_examples, rel_tol=0.0, abs_tol=0.0)

    def test_eight_decimal_places(self):
        # examples that are close to 1e-8, but not 1e-9
        eight_decimal_places_examples = [(1e8, 1e8 + 1),
                                         (-1e-8, -1.000000009e-8),
                                         (1.12345678, 1.12345679)]
        self.assertAllClose(eight_decimal_places_examples, rel_tol=1e-8)
        self.assertAllNotClose(eight_decimal_places_examples, rel_tol=1e-9)

    def test_near_zero(self):
        # values close to zero
        near_zero_examples = [(1e-9, 0.0),
                              (-1e-9, 0.0),
                              (-1e-150, 0.0)]
        # these should not be close to any rel_tol
        self.assertAllNotClose(near_zero_examples, rel_tol=0.9)
        # these should be close to abs_tol=1e-8
        self.assertAllClose(near_zero_examples, abs_tol=1e-8)

    def test_identical_infinite(self):
        # these are close regardless of tolerance -- i.e. they are equal
        self.assertIsClose(INF, INF)
        self.assertIsClose(INF, INF, abs_tol=0.0)
        self.assertIsClose(NINF, NINF)
        self.assertIsClose(NINF, NINF, abs_tol=0.0)

    def test_inf_ninf_nan(self):
        # these should never be close (following IEEE 754 rules for equality)
        not_close_examples = [(NAN, NAN),
                              (NAN, 1e-100),
                              (1e-100, NAN),
                              (INF, NAN),
                              (NAN, INF),
                              (INF, NINF),
                              (INF, 1.0),
                              (1.0, INF),
                              (INF, 1e308),
                              (1e308, INF)]
        # use largest reasonable tolerance
        self.assertAllNotClose(not_close_examples, abs_tol=0.999999999999999)

    def test_zero_tolerance(self):
        # test with zero tolerance
        zero_tolerance_close_examples = [(1.0, 1.0),
                                         (-3.4, -3.4),
                                         (-1e-300, -1e-300)]
        self.assertAllClose(zero_tolerance_close_examples, rel_tol=0.0)

        zero_tolerance_not_close_examples = [(1.0, 1.000000000000001),
                                             (0.99999999999999, 1.0),
                                             (1.0e200, .999999999999999e200)]
        self.assertAllNotClose(zero_tolerance_not_close_examples, rel_tol=0.0)

    def test_asymmetry(self):
        # test the asymmetry example from PEP 485
        self.assertAllClose([(9, 10), (10, 9)], rel_tol=0.1)

    def test_integers(self):
        # test with integer values
        integer_examples = [(100000001, 100000000),
                            (123456789, 123456788)]

        self.assertAllClose(integer_examples, rel_tol=1e-8)
        self.assertAllNotClose(integer_examples, rel_tol=1e-9)

    def test_decimals(self):
        # test with Decimal values
        from decimal import Decimal

        decimal_examples = [(Decimal('1.00000001'), Decimal('1.0')),
                            (Decimal('1.00000001e-20'), Decimal('1.0e-20')),
                            (Decimal('1.00000001e-100'), Decimal('1.0e-100')),
                            (Decimal('1.00000001e20'), Decimal('1.0e20'))]
        self.assertAllClose(decimal_examples, rel_tol=1e-8)
        self.assertAllNotClose(decimal_examples, rel_tol=1e-9)

    def test_fractions(self):
        # test with Fraction values
        from fractions import Fraction

        fraction_examples = [
            (Fraction(1, 100000000) + 1, Fraction(1)),
            (Fraction(100000001), Fraction(100000000)),
            (Fraction(10**8 + 1, 10**28), Fraction(1, 10**20))]
        self.assertAllClose(fraction_examples, rel_tol=1e-8)
        self.assertAllNotClose(fraction_examples, rel_tol=1e-9)
