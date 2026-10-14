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


# The persistent map CPython exposes as _testinternalcapi.hamt, the store
# behind a context. set and delete hand back the same object when nothing
# would change (the stored value is that same object, or the key is
# absent) and a fresh map otherwise. Lookup goes through hash and
# equality and lets an error from either one out. An iterator keeps its
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
            if node[2] == key:
                if node[3] is value:
                    return node, False
                return ('L', hashed, node[2], value), False
            return ('C', hashed, ((node[2], node[3]), (key, value))), True
        return _join(node, ('L', hashed, key, value), shift), True
    if kind == 'C':
        if node[1] != hashed:
            return _join(node, ('L', hashed, key, value), shift), True
        found = False
        pairs = []
        for old_key, old_value in node[2]:
            if old_key == key:
                found = True
                if old_value is value:
                    return node, False
                pairs.append((old_key, value))
            else:
                pairs.append((old_key, old_value))
        if not found:
            pairs.append((key, value))
            return ('C', hashed, tuple(pairs)), True
        return ('C', hashed, tuple(pairs)), False
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
        if node[1] == hashed and node[2] == key:
            return None, True
        return node, False
    if kind == 'C':
        if node[1] != hashed:
            return node, False
        kept = []
        found = False
        for old_key, old_value in node[2]:
            if not found and old_key == key:
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
        raise KeyError(key)
    kind = node[0]
    if kind == 'L':
        if node[1] == hashed and node[2] == key:
            return node[3]
        raise KeyError(key)
    if kind == 'C':
        if node[1] != hashed:
            raise KeyError(key)
        for old_key, old_value in node[2]:
            if old_key == key:
                return old_value
        raise KeyError(key)
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
        try:
            return _find(self._root, key, _hash_bits(key), 0)
        except KeyError:
            return default

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
        try:
            _find(self._root, key, _hash_bits(key), 0)
        except KeyError:
            return False
        return True

    def __getitem__(self, key):
        return _find(self._root, key, _hash_bits(key), 0)

    def __eq__(self, other):
        if type(other) is not hamt:
            return NotImplemented
        if self._len != other._len:
            return False
        pairs = []
        _walk(self._root, pairs)
        for key, value in pairs:
            try:
                found = _find(other._root, key, _hash_bits(key), 0)
            except KeyError:
                return False
            if found != value:
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
