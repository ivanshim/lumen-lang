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


# The persistent map CPython exposes as _testinternalcapi.hamt, the store
# behind a context. set and delete hand back the same object when nothing
# would change (the stored value is that same object, or the key is
# absent) and a fresh map otherwise. A stored key matches itself before
# equality is asked, so a key that is not equal to itself is still found,
# and the lookup key is the left operand of that comparison. Hashing and
# equality errors escape. An empty map has no entry, so get and
# membership do not hash. An iterator keeps its
# map alive and reports the length the map had when the iterator was
# made, so a cycle that runs through the iterator can be collected.
#
# The map is a hash-array trie: each level takes five bits of the key's
# hash, and only the nodes on the edited path are rebuilt. Keys whose
# hashes are equal live together in one collision list.


_MASK64 = (1 << 64) - 1
_FAN = 32
_SLOT = _FAN - 1


def _hash_bits(key):
    return hash(key) & _MASK64


# Not a KeyError: a key callback may raise KeyError, and that has to
# reach the caller. Lookup hands this back when the key is not there.
_MISSING = object()


def _same_key(stored, key):
    return stored is key or key == stored


def _empty_slots():
    return [None] * _FAN


def _write_slot(children, index, child):
    slots = _empty_slots() if children is None else list(children)
    slots[index] = child
    return ('B', tuple(slots))


def _join(left, right, shift):
    if shift >= 64 or left[1] == right[1]:
        pairs = []
        for node in (left, right):
            if node[0] == 'C':
                pairs.extend(node[2])
            else:
                pairs.append((node[2], node[3]))
        return ('C', left[1], tuple(pairs))
    left_i = (left[1] >> shift) & _SLOT
    right_i = (right[1] >> shift) & _SLOT
    if left_i == right_i:
        return _write_slot(None, left_i, _join(left, right, shift + 5))
    slots = _empty_slots()
    slots[left_i] = left
    slots[right_i] = right
    return ('B', tuple(slots))


def _put(node, key, value, hashed, shift):
    if node is None:
        return ('L', hashed, key, value), True
    kind = node[0]
    if kind == 'L':
        if node[1] == hashed:
            if _same_key(node[2], key):
                if node[3] is value:
                    return node, False
                return ('L', hashed, node[2], value), False
            return ('C', hashed, ((node[2], node[3]), (key, value))), True
        return _join(node, ('L', hashed, key, value), shift), True
    if kind == 'C':
        if node[1] != hashed:
            return _join(node, ('L', hashed, key, value), shift), True
        entries = node[2]
        for at in range(len(entries)):
            old_key, old_value = entries[at]
            if _same_key(old_key, key):
                if old_value is value:
                    return node, False
                return ('C', hashed, entries[:at] + ((old_key, value),) + entries[at + 1:]), False
        return ('C', hashed, entries + ((key, value),)), True
    index = (hashed >> shift) & _SLOT
    children = node[1]
    child = children[index]
    new_child, grew = _put(child, key, value, hashed, shift + 5)
    if new_child is child:
        return node, False
    return _write_slot(children, index, new_child), grew


def _delete(node, key, hashed, shift):
    if node is None:
        return None, False
    kind = node[0]
    if kind == 'L':
        if node[1] == hashed and _same_key(node[2], key):
            return None, True
        return node, False
    if kind == 'C':
        if node[1] != hashed:
            return node, False
        kept = []
        found = False
        for old_key, old_value in node[2]:
            if not found and _same_key(old_key, key):
                found = True
            else:
                kept.append((old_key, old_value))
        if not found:
            return node, False
        if len(kept) == 1:
            only_key, only_value = kept[0]
            return ('L', hashed, only_key, only_value), True
        return ('C', hashed, tuple(kept)), True
    index = (hashed >> shift) & _SLOT
    children = node[1]
    new_child, shrank = _delete(children[index], key, hashed, shift + 5)
    if not shrank:
        return node, False
    return _write_slot(children, index, new_child), True


def _find(node, key, hashed, shift):
    if node is None:
        return _MISSING
    kind = node[0]
    if kind == 'L':
        if node[1] == hashed and _same_key(node[2], key):
            return node[3]
        return _MISSING
    if kind == 'C':
        if node[1] != hashed:
            return _MISSING
        for old_key, old_value in node[2]:
            if _same_key(old_key, key):
                return old_value
        return _MISSING
    index = (hashed >> shift) & _SLOT
    return _find(node[1][index], key, hashed, shift + 5)


def _walk(node, out):
    if node is None:
        return
    kind = node[0]
    if kind == 'L':
        out.append((node[2], node[3]))
    elif kind == 'C':
        out.extend(node[2])
    else:
        for child in node[1]:
            if child is not None:
                _walk(child, out)


class _HamtIter:
    def __init__(self, owner, kind):
        self._owner = owner
        self._kind = kind
        pairs = []
        _walk(owner._root, pairs)
        self._pairs = pairs
        self._n = len(pairs)
        self._i = 0

    def __iter__(self):
        return self

    def __len__(self):
        return self._n

    def __next__(self):
        if self._i >= self._n:
            raise StopIteration
        key, value = self._pairs[self._i]
        self._i += 1
        if self._kind == 'items':
            return (key, value)
        if self._kind == 'keys':
            return key
        return value


class hamt:
    def __init__(self):
        self._root = None
        self._len = 0

    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'hamt' is not an acceptable base type")

    def _spawn(self, root, size):
        born = object.__new__(hamt)
        born._root = root
        born._len = size
        return born

    def set(self, key, value):
        hashed = _hash_bits(key)
        root, grew = _put(self._root, key, value, hashed, 0)
        if root is self._root:
            return self
        return self._spawn(root, self._len + (1 if grew else 0))

    def get(self, key, default=None):
        if self._root is None:
            return default
        found = _find(self._root, key, _hash_bits(key), 0)
        if found is _MISSING:
            return default
        return found

    def delete(self, key):
        hashed = _hash_bits(key)
        root, shrank = _delete(self._root, key, hashed, 0)
        if not shrank:
            return self
        return self._spawn(root, self._len - 1)

    def __len__(self):
        return self._len

    def __iter__(self):
        return _HamtIter(self, 'keys')

    def __contains__(self, key):
        if self._root is None:
            return False
        found = _find(self._root, key, _hash_bits(key), 0)
        return found is not _MISSING

    def __getitem__(self, key):
        if self._root is None:
            raise KeyError(key)
        found = _find(self._root, key, _hash_bits(key), 0)
        if found is _MISSING:
            raise KeyError(key)
        return found

    def __eq__(self, other):
        if type(other) is not hamt:
            return NotImplemented
        if self is other:
            return True
        if self._len != other._len:
            return False
        pairs = []
        _walk(self._root, pairs)
        for key, value in pairs:
            found = _find(other._root, key, _hash_bits(key), 0)
            if found is _MISSING:
                return False
            if not (found is value or found == value):
                return False
        return True

    def items(self):
        return _HamtIter(self, 'items')

    def keys(self):
        return _HamtIter(self, 'keys')

    def values(self):
        return _HamtIter(self, 'values')


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


# Native nested runtimes have independent modules and globals. Allocator and
# GIL selectors do not change that state in this single-threaded Rust runtime;
# fork, exec and starting OS threads are unavailable in these adapters.
def run_in_subinterp_with_config(code, config, /):
    for name in ('allow_fork', 'allow_exec', 'allow_threads',
                 'allow_daemon_threads', 'use_main_obmalloc',
                 'check_multi_interp_extensions'):
        if not isinstance(getattr(config, name), int):
            raise TypeError(name + ' must be an integer')
    if config.gil not in (0, 1, 2, 'default', 'shared', 'own'):
        raise ValueError('invalid GIL selector')
    import _testcapi
    return _testcapi.run_in_subinterp(code)
