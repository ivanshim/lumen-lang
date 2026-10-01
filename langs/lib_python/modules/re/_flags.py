# Integer flag adapter for the embedded re package.
_NAMES = [(256, 'ASCII'), (2, 'IGNORECASE'), (4, 'LOCALE'), (32, 'UNICODE'),
          (8, 'MULTILINE'), (16, 'DOTALL'), (64, 'VERBOSE'), (128, 'DEBUG')]
class RegexFlag(int):
    __module__ = 're'
    def __new__(cls, value=0):
        value = int(value)
        if value < 0:
            value = value & ((1 << max(9, (~value).bit_length())) - 1)
        return int.__new__(cls, value)
    @property
    def value(self):
        return int(self)
    @property
    def name(self):
        if not self:
            return 'NOFLAG'
        return '|'.join(name for n, name in _NAMES if self & n)
    def __repr__(self):
        if not self:
            return 're.NOFLAG'
        parts = ['re.' + name for n, name in _NAMES if int(self) & n]
        unknown = int(self) & ~510
        if unknown:
            parts.append(hex(unknown))
        return '|'.join(parts)
    __str__ = __repr__
    def __or__(self, other):
        return RegexFlag(int(self) | int(other))
    __ror__ = __or__
    def __and__(self, other):
        return RegexFlag(int(self) & int(other))
    __rand__ = __and__
    def __xor__(self, other):
        return RegexFlag(int(self) ^ int(other))
    __rxor__ = __xor__
    def __invert__(self):
        return RegexFlag(~int(self) & ((1 << max(9, int(self).bit_length())) - 1))
NOFLAG = RegexFlag(0)
ASCII = RegexFlag(256)
IGNORECASE = RegexFlag(2)
LOCALE = RegexFlag(4)
UNICODE = RegexFlag(32)
MULTILINE = RegexFlag(8)
DOTALL = RegexFlag(16)
VERBOSE = RegexFlag(64)
DEBUG = RegexFlag(128)
for _value, _name in _NAMES:
    setattr(RegexFlag, _name, RegexFlag(_value))
RegexFlag.NOFLAG = NOFLAG
A = ASCII
I = IGNORECASE
L = LOCALE
U = UNICODE
M = MULTILINE
S = DOTALL
X = VERBOSE
__all__ = ['RegexFlag', 'NOFLAG', 'ASCII', 'IGNORECASE', 'LOCALE', 'UNICODE',
           'MULTILINE', 'DOTALL', 'VERBOSE', 'DEBUG', 'A', 'I', 'L', 'U', 'M', 'S', 'X']
