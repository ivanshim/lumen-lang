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
    import sys
    frame = sys._getframe(1)
    depth = 0
    while frame is not None:
        depth += 1
        frame = frame.f_back
    return depth


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
