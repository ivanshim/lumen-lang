# Runtime bridges for the supported interpreter configuration and display APIs.
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
