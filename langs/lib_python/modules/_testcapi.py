# Runtime bridges for the supported interpreter configuration and display APIs.
import os

# Read the debug-range setting that the kernels use for position records.
def config_get(name):
    if name != 'code_debug_ranges':
        raise NotImplementedError('this interpreter configuration field is unavailable')
    setting = os.environ.get('PYTHONNODEBUGRANGES', '0')
    try:
        disabled = int(setting or '0') != 0
    except ValueError:
        disabled = False
    return not disabled

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
FLT_MAX = 3.4028234663852886e+38

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
