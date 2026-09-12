# The numeric tower: each kind stands on the one above it, and a kind
# declares the operations a number of that kind answers to.
#
# Stub: register records the claim and answers with the class it was
# given, as CPython's does, but isinstance cannot honour it. The reader
# answers isinstance from the class a thing was built from and offers no
# instance-check hook, so isinstance(1, Integral) is false here where
# CPython says true, and a registered class is known only through the
# registry below.
from abc import ABC, abstractmethod

__all__ = ['Number', 'Complex', 'Real', 'Rational', 'Integral']

# Which classes have been claimed for which kind, by the kind's name.
_claimed = {}


class Number(ABC):
    @classmethod
    def register(cls, subclass):
        table = _claimed
        name = cls.__name__
        if name not in table:
            table[name] = []
        if subclass not in table[name]:
            table[name].append(subclass)
        return subclass

    @classmethod
    def _registered(cls):
        table = _claimed
        name = cls.__name__
        if name not in table:
            return []
        return list(table[name])

    # CPython makes a number unhashable by default, by giving the kind
    # no hash at all. The reader has no way to take a hash away, so the
    # kind's own hash refuses instead.
    def __hash__(self):
        raise NotImplementedError('__hash__')


class Complex(Number):
    @abstractmethod
    def __complex__(self):
        raise NotImplementedError('__complex__')

    def __bool__(self):
        return self != 0

    @property
    @abstractmethod
    def real(self):
        raise NotImplementedError('real')

    @property
    @abstractmethod
    def imag(self):
        raise NotImplementedError('imag')

    @abstractmethod
    def __add__(self, other):
        return NotImplemented

    @abstractmethod
    def __radd__(self, other):
        return NotImplemented

    @abstractmethod
    def __neg__(self):
        raise NotImplementedError('__neg__')

    @abstractmethod
    def __pos__(self):
        raise NotImplementedError('__pos__')

    def __sub__(self, other):
        return self + -other

    def __rsub__(self, other):
        return -self + other

    @abstractmethod
    def __mul__(self, other):
        return NotImplemented

    @abstractmethod
    def __rmul__(self, other):
        return NotImplemented

    @abstractmethod
    def __truediv__(self, other):
        return NotImplemented

    @abstractmethod
    def __rtruediv__(self, other):
        return NotImplemented

    @abstractmethod
    def __pow__(self, exponent):
        return NotImplemented

    @abstractmethod
    def __rpow__(self, base):
        return NotImplemented

    @abstractmethod
    def __abs__(self):
        raise NotImplementedError('__abs__')

    @abstractmethod
    def conjugate(self):
        raise NotImplementedError('conjugate')

    @abstractmethod
    def __eq__(self, other):
        return NotImplemented


class Real(Complex):
    @abstractmethod
    def __float__(self):
        raise NotImplementedError('__float__')

    @abstractmethod
    def __trunc__(self):
        raise NotImplementedError('__trunc__')

    @abstractmethod
    def __floor__(self):
        raise NotImplementedError('__floor__')

    @abstractmethod
    def __ceil__(self):
        raise NotImplementedError('__ceil__')

    @abstractmethod
    def __round__(self, digits=None):
        raise NotImplementedError('__round__')

    def __divmod__(self, other):
        return (self // other, self % other)

    def __rdivmod__(self, other):
        return (other // self, other % self)

    @abstractmethod
    def __floordiv__(self, other):
        return NotImplemented

    @abstractmethod
    def __rfloordiv__(self, other):
        return NotImplemented

    @abstractmethod
    def __mod__(self, other):
        return NotImplemented

    @abstractmethod
    def __rmod__(self, other):
        return NotImplemented

    @abstractmethod
    def __lt__(self, other):
        return NotImplemented

    @abstractmethod
    def __le__(self, other):
        return NotImplemented

    def __complex__(self):
        return complex(float(self), 0.0)

    @property
    def real(self):
        return +self

    @property
    def imag(self):
        return 0

    def conjugate(self):
        return +self


class Rational(Real):
    @property
    @abstractmethod
    def numerator(self):
        raise NotImplementedError('numerator')

    @property
    @abstractmethod
    def denominator(self):
        raise NotImplementedError('denominator')

    def __float__(self):
        return int(self.numerator) / int(self.denominator)


class Integral(Rational):
    @abstractmethod
    def __int__(self):
        raise NotImplementedError('__int__')

    def __index__(self):
        return int(self)

    @abstractmethod
    def __pow__(self, exponent, modulus=None):
        return NotImplemented

    @abstractmethod
    def __lshift__(self, other):
        return NotImplemented

    @abstractmethod
    def __rlshift__(self, other):
        return NotImplemented

    @abstractmethod
    def __rshift__(self, other):
        return NotImplemented

    @abstractmethod
    def __rrshift__(self, other):
        return NotImplemented

    @abstractmethod
    def __and__(self, other):
        return NotImplemented

    @abstractmethod
    def __rand__(self, other):
        return NotImplemented

    @abstractmethod
    def __xor__(self, other):
        return NotImplemented

    @abstractmethod
    def __rxor__(self, other):
        return NotImplemented

    @abstractmethod
    def __or__(self, other):
        return NotImplemented

    @abstractmethod
    def __ror__(self, other):
        return NotImplemented

    @abstractmethod
    def __invert__(self):
        raise NotImplementedError('__invert__')

    def __float__(self):
        return float(int(self))

    @property
    def numerator(self):
        return +self

    @property
    def denominator(self):
        return 1


# The builtin numbers, claimed for the kinds they answer to.
Integral.register(int)
Real.register(float)
Complex.register(complex)
