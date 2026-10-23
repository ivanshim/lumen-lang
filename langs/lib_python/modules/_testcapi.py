# Runtime bridges for the supported interpreter configuration and display APIs.
import os
import sys

# Expose the signed size limit used by native sequence lengths.
PY_SSIZE_T_MAX = sys.maxsize
from _code_metadata import _debug_ranges_enabled

# Read the debug-range setting that the kernels use for position records.
def config_get(name):
    if name != 'code_debug_ranges':
        raise NotImplementedError('this interpreter configuration field is unavailable')
    return _debug_ranges_enabled()


# Display an actual exception through the standard formatter and stderr policy.
def exception_print(exception):
    import sys
    import traceback
    import _colorize
    if not isinstance(exception, BaseException):
        raise TypeError('an exception instance is required')
    colorize = _colorize.can_colorize(file=sys.stderr)
    traceback.TracebackException.from_exception(exception).print(file=sys.stderr, colorize=colorize)

# These bridges preserve the Python-visible operations of the C test helpers.
def bad_get(self, obj, cls):
    cls()
    return repr(self)

class HeapCTypeMetaclassNullNew(type):
    def __new__(cls, *args, **kwargs):
        raise TypeError("cannot create '" + cls.__name__ + "' instances")

def pytype_fromspec_meta(metaclass):
    return type.__new__(metaclass, 'HeapCType', (object,), {})

INT_MIN = -(1 << 31)
INT_MAX = (1 << 31) - 1
LONG_MIN = -(1 << 63)
LONG_MAX = (1 << 63) - 1
ULONG_MAX = (1 << 64) - 1

def PyTime_AsSecondsDouble(value):
    import operator
    value = operator.index(value)
    if not -(1 << 63) <= value < (1 << 63):
        raise OverflowError('timestamp out of range for C PyTime_t')
    return float(value // 1000000000) if value % 1000000000 == 0 else float(value) / 1000000000


def run_in_subinterp(code):
    if not isinstance(code, str):
        raise TypeError('code must be a string')
    return __program_namespace('subinterp', code)

# Compile a source string with the C helper's input-mode and encoding flags.
def Py_CompileStringExFlags(source, filename, start, flags=0, optimize=-1):
    import operator
    start = operator.index(start)
    flags = operator.index(flags)
    modes = {256: 'single', 257: 'exec', 258: 'eval'}
    if start not in modes:
        raise ValueError('invalid input mode')
    if flags & 0x0800:
        if isinstance(source, bytes):
            source = source.decode('utf-8')
        flags &= ~0x0800
    return compile(source, filename, modes[start], flags=flags,
                   dont_inherit=True, optimize=optimize)

# This interpreter uses IEEE-754 quiet NaNs: the most significant
# fraction bit is set. Platforms that invert that bit use 1.
nan_msb_is_signaling = 0
