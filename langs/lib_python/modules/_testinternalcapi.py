# What the reference implementation's own internals answer about its
# objects. Only the probes this runtime can answer truthfully stand
# here; the rest refuse at the bottom of the file.

import signal as _signal

def has_inline_values(obj):
    # The kernel tracks the compact layout until dictionary replacement,
    # deletion, or growth beyond its available attribute places.
    return _has_inline_values(obj)


class SelfInterruptingContextManager:
    # The reference's own is written in C, where no evaluation check
    # stands between its words: it leaves an interrupt pending as it
    # enters, so the KeyboardInterrupt lands on the first statement of
    # the body and the leaving method runs all the same. The same is
    # arranged here by shape: the raise is the entering method's last
    # statement, so no statement's edge stands between the pending note
    # and the frame's end, and the interrupt is taken up as the body
    # begins.
    def __init__(self):
        self._within = False

    def __enter__(self):
        self._within = True
        _signal.raise_signal(_signal.SIGINT)

    def __exit__(self, *args):
        self._within = False
        return False

    def within(self):
        return self._within


# The reference's own module reports this many runs before a
# specialization is made; the same number stands here so the runs the
# tests make of it run as they run there.
SPECIALIZATION_THRESHOLD = 2


def get_recursion_depth():
    # Use the same logical depth as the kernel's recursion-limit check.
    return __program_namespace('recursion_depth') - 1


# Contracts of CPython v3.14.8 Modules/_testinternalcapi.c and fileutils.c.
def _locale_arguments(current_locale, errors):
    import operator
    current_locale = operator.index(current_locale)
    if current_locale < -2147483648:
        raise OverflowError('signed integer is less than minimum')
    if current_locale > 2147483647:
        raise OverflowError('signed integer is greater than maximum')
    if not isinstance(errors, str):
        raise TypeError('argument 3 must be str, not ' + type(errors).__name__)
    if '\0' in errors:
        raise ValueError('embedded null character')
    import sys
    encoding = __posix('locale_encoding')[1].decode('ascii') if current_locale else sys.getfilesystemencoding()
    allowed = ('strict', 'surrogateescape', 'surrogatepass') if not current_locale and encoding.lower().replace('_', '-') == 'utf-8' else ('strict', 'surrogateescape')
    if errors not in allowed:
        raise ValueError('unsupported error handler')
    return encoding, current_locale


def EncodeLocaleEx(text, current_locale=0, errors='strict', /):
    if not isinstance(text, str):
        raise TypeError('argument 1 must be str, not ' + type(text).__name__)
    if '\0' in text:
        raise ValueError('embedded null character')
    encoding, current_locale = _locale_arguments(current_locale, errors)
    try:
        return text.encode(encoding, errors)
    except UnicodeEncodeError as exc:
        raise RuntimeError('encode error: pos=%d, reason=%s' % (exc.start, 'encoding error')) from None


def DecodeLocaleEx(data, current_locale=0, errors='strict', /):
    if not isinstance(data, (bytes, bytearray, memoryview)):
        raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'")
    if not isinstance(data, bytes):
        raise TypeError('argument 1 must be read-only bytes-like object, not ' + type(data).__name__)
    if b'\0' in data:
        raise ValueError('embedded null byte')
    encoding, current_locale = _locale_arguments(current_locale, errors)
    try:
        return data.decode(encoding, errors)
    except UnicodeDecodeError as exc:
        raise RuntimeError('decode error: pos=%d, reason=%s' % (exc.start, 'decoding error' if current_locale else exc.reason)) from None


def __getattr__(name):
    raise AttributeError("module '_testinternalcapi' has no attribute '" + name + "'")

# CPython's signed nanosecond and time_t conversion contracts.
SIZEOF_TIME_T = 8

def _time_bound(value, bits=64, kind='C PyTime_t'):
    if not -(1 << (bits-1)) <= value < (1 << (bits-1)):
        raise OverflowError('timestamp out of range for ' + kind)
    return value

def _time_round(value, rounding):
    import math
    if math.isnan(value):
        raise ValueError('Invalid value NaN (not a number)')
    if not math.isfinite(value):
        raise OverflowError('timestamp out of range for C PyTime_t')
    if rounding == 0: return math.floor(value)
    if rounding == 1: return math.ceil(value)
    if rounding == 2: return round(value)
    if rounding == 3: return math.floor(value) if value < 0 else math.ceil(value)
    raise ValueError('invalid rounding mode')

def _time_index(value):
    import operator
    return operator.index(value)

def _time_divide(value, divisor, rounding):
    value = _time_bound(_time_index(value))
    q, r = divmod(value, divisor)
    if rounding == 0: return q
    if rounding == 1: return q + bool(r)
    if rounding == 2: return q + (r*2 > divisor or (r*2 == divisor and q % 2 != 0))
    if rounding == 3: return q if value < 0 else q + bool(r)
    raise ValueError('invalid rounding mode')

def _PyTime_FromSeconds(value):
    return _time_bound(_time_bound(_time_index(value), 32, 'C int') * 1000000000)

def _PyTime_FromSecondsObject(value, rounding):
    ns = _time_round(value * 1000000000, rounding) if isinstance(value, float) else _time_index(value) * 1000000000
    return _time_bound(ns)

def _PyTime_AsMilliseconds(value, rounding):
    return _time_divide(value, 1000000, rounding)

def _PyTime_AsMicroseconds(value, rounding):
    return _time_divide(value, 1000, rounding)

def _PyTime_AsTimeval(value, rounding):
    return divmod(_PyTime_AsMicroseconds(value, rounding), 1000000)

def _time_object_parts(value, rounding, scale):
    import math
    if isinstance(value, float):
        fraction, seconds = math.modf(value)
        seconds = _time_round(seconds, rounding)
        units = _time_round(fraction * scale, rounding)
        carry, units = divmod(units, scale)
        seconds += carry
    else:
        seconds, units = _time_index(value), 0
    return (_time_bound(seconds, 64, 'platform time_t'), units)

def _PyTime_ObjectToTime_t(value, rounding):
    return _time_bound(_time_round(value, rounding) if isinstance(value, float) else _time_index(value), 64, 'platform time_t')

def _PyTime_ObjectToTimeval(value, rounding):
    return _time_object_parts(value, rounding, 1000000)

def _PyTime_ObjectToTimespec(value, rounding):
    return _time_object_parts(value, rounding, 1000000000)
