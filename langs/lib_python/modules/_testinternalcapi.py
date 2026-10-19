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

# These code analyses read the kernels' native instructions and scope layout.
def _code_subject(value):
    import types
    if isinstance(value, types.CodeType):
        return value, None
    if isinstance(value, types.FunctionType):
        return value.__code__, value
    raise TypeError('argument must be a code object or a function')


def code_returns_only_none(code, /):
    import types
    if not isinstance(code, types.CodeType):
        raise TypeError('argument must be a code object')
    return not bool(code.co_flags & (0x20 | 0x80 | 0x100 | 0x200)) and __trace_native__(11, code)[2]


def get_co_localskinds(value, /):
    import types
    if not isinstance(value, types.CodeType):
        raise TypeError('argument must be a code object')
    code = value
    kinds = {}
    argcount = code.co_argcount
    keyword_end = argcount + code.co_kwonlyargcount
    for index, name in enumerate(code.co_varnames):
        kind = 0x20
        if index < code.co_posonlyargcount:
            kind |= 0x02
        elif index < argcount:
            kind |= 0x06
        elif index < keyword_end:
            kind |= 0x04
        elif index == keyword_end and code.co_flags & 4:
            kind |= 0x0a
        elif index == keyword_end + bool(code.co_flags & 4) and code.co_flags & 8:
            kind |= 0x0c
        kinds[name] = kind
    for name in code.co_cellvars:
        kinds[name] = kinds.get(name, 0) | 0x40
    for name in code.co_freevars:
        kinds[name] = 0x80
    return kinds


_UNSPECIFIED_CODE_NAMESPACE = object()


def _code_namespace(value, position):
    if value is not _UNSPECIFIED_CODE_NAMESPACE and not isinstance(value, dict):
        actual = 'None' if value is None else type(value).__name__
        raise TypeError('get_code_var_counts() argument ' + str(position) + ' must be dict, not ' + actual)


def get_code_var_counts(code, globalnames=_UNSPECIFIED_CODE_NAMESPACE,
                        attrnames=_UNSPECIFIED_CODE_NAMESPACE,
                        globalsns=_UNSPECIFIED_CODE_NAMESPACE,
                        builtinsns=_UNSPECIFIED_CODE_NAMESPACE):
    for namespace, position in ((globalsns, 4), (builtinsns, 5)):
        _code_namespace(namespace, position)
    code, function = _code_subject(code)
    kinds = get_co_localskinds(code)
    references, attributes, _ = __trace_native__(11, code)
    for names, label in ((globalnames, 'globalnames'), (attrnames, 'attrnames')):
        if names is not _UNSPECIFIED_CODE_NAMESPACE and not isinstance(names, set):
            raise TypeError('expected a set for "' + label + '", got ' + repr(names))
    if globalnames is _UNSPECIFIED_CODE_NAMESPACE:
        globalnames = set()
    if attrnames is _UNSPECIFIED_CODE_NAMESPACE:
        attrnames = set()
    references, attributes = set(references), set(attributes)
    globals_ = references - globalnames
    attrs = attributes - attrnames
    duplicates = len(((references | globalnames) & (attributes | attrnames)) & (globals_ | attrs))
    globalnames.update(references)
    attrnames.update(attributes)
    if globalsns is _UNSPECIFIED_CODE_NAMESPACE:
        globalsns = {} if function is None else function.__globals__
    if builtinsns is _UNSPECIFIED_CODE_NAMESPACE:
        builtinsns = {} if function is None else function.__builtins__
    args = code.co_argcount + code.co_kwonlyargcount + bool(code.co_flags & 4) + bool(code.co_flags & 8)
    argnames = code.co_varnames[:args]
    argcells = sum(name in code.co_cellvars for name in argnames)
    cells = len(code.co_cellvars)
    pure = len(code.co_varnames) - args
    othercells = cells - argcells
    locals_ = args + pure + othercells
    global_count = sum(name in globalsns for name in globals_)
    builtin_count = sum(name not in globalsns and name in builtinsns for name in globals_)
    unbound = len(code.co_names) + duplicates
    unknown = unbound - len(globals_) - len(attrs)
    return {
        'total': locals_ + len(code.co_freevars) + unbound,
        'locals': {
            'total': locals_,
            'args': {'total': args, 'numposonly': code.co_posonlyargcount,
                     'numposorkw': code.co_argcount - code.co_posonlyargcount,
                     'numkwonly': code.co_kwonlyargcount,
                     'varargs': int(bool(code.co_flags & 4)),
                     'varkwargs': int(bool(code.co_flags & 8))},
            'numpure': pure,
            'cells': {'total': cells, 'numargs': argcells, 'numothers': othercells},
            'hidden': {'total': sum(bool(kind & 0x10) for kind in kinds.values()),
                       'numpure': sum(bool(kind & 0x10) and not bool(kind & 0x40) for kind in kinds.values()),
                       'numcells': sum(bool(kind & 0x10) and bool(kind & 0x40) for kind in kinds.values())},
        },
        'numfree': len(code.co_freevars),
        'unbound': {
            'total': unbound,
            'numattrs': len(attrs), 'numunknown': unknown,
            'globals': {'total': len(globals_), 'numglobal': global_count,
                        'numbuiltin': builtin_count,
                        'numunknown': len(globals_) - global_count - builtin_count},
        },
    }


def verify_stateless_code(code, globalnames=_UNSPECIFIED_CODE_NAMESPACE,
                          globalsns=_UNSPECIFIED_CODE_NAMESPACE,
                          builtinsns=_UNSPECIFIED_CODE_NAMESPACE):
    if globalnames is not _UNSPECIFIED_CODE_NAMESPACE and not isinstance(globalnames, set):
        actual = 'None' if globalnames is None else type(globalnames).__name__
        raise TypeError('get_code_var_counts() argument 2 must be set, not ' + actual)
    for namespace, position in ((globalsns, 3), (builtinsns, 4)):
        _code_namespace(namespace, position)
    code, function = _code_subject(code)
    subject = code if function is None else function
    counts = get_code_var_counts(subject, globalnames=globalnames,
                                 globalsns=globalsns, builtinsns=builtinsns)
    if code.co_freevars:
        raise ValueError('closures not supported')
    globals_ = counts['unbound']['globals']
    has_builtins = function is not None or builtinsns is not _UNSPECIFIED_CODE_NAMESPACE
    if globals_['numglobal'] or (has_builtins and globals_['numunknown']):
        raise ValueError('globals not supported')
    return None
