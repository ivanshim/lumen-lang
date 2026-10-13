# A value handed out as bytes and taken back as itself. The writing is
# this runtime's own and not the reference implementation's marshal
# format, so what is written here is read back only here; it says so
# plainly rather than pretending to a format it does not have. What it
# does keep is what marshal promises: the built-in kinds and nothing of
# a class of its own, the two singletons back as the singletons, and a
# thing that stood in two places standing in two places again.
#
# The writing itself is lent to pickle, which has the same work to do
# and takes a slice as well; see _write and _read at the foot of the
# file.

import struct


class _Writer:
    def __init__(self, slices, refuse):
        self.pieces = []
        self.memo = {}
        self.kept = []
        self.slices = slices
        self.refuse = refuse

    def text(self):
        return ''.join(self.pieces)

    def held(self, value):
        # A thing already written stands again as the number of its
        # place. Every container takes a place, and takes it before its
        # contents, so that a thing holding itself is read back.
        where = id(value)
        if where in self.memo:
            self.pieces.append('b' + str(self.memo[where][0]) + ';')
            return True
        # Retain temporary reduction tuples until writing finishes: an
        # address alone can be reused for a different object meanwhile.
        self.memo[where] = (len(self.memo), value)
        self.kept.append(value)
        return False

    def put(self, value):
        if value is None:
            self.pieces.append('n')
            return
        if value is True:
            self.pieces.append('T')
            return
        if value is False:
            self.pieces.append('F')
            return
        if isinstance(value, int):
            self.exactly(value, type(value) is int)
            self.pieces.append('i' + str(value) + ';')
            return
        if isinstance(value, float):
            self.exactly(value, type(value) is float)
            self.pieces.append('r' + repr(value) + ';')
            return
        if isinstance(value, complex):
            self.exactly(value, type(value) is complex)
            self.pieces.append('x' + repr(value.real) + ';' + repr(value.imag) + ';')
            return
        if isinstance(value, str):
            self.exactly(value, type(value) is str)
            self.pieces.append('u' + str(len(value)) + ';' + value)
            return
        if isinstance(value, bytes):
            self.exactly(value, type(value) is bytes)
            self.pieces.append('y' + str(len(value)) + ';' + value.hex())
            return
        if isinstance(value, tuple):
            self.exactly(value, type(value) is tuple)
            if self.held(value):
                return
            self.pieces.append('P' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        if isinstance(value, list):
            self.exactly(value, type(value) is list)
            if self.held(value):
                return
            self.pieces.append('L' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        if isinstance(value, dict):
            self.exactly(value, type(value) is dict)
            if self.held(value):
                return
            self.pieces.append('D' + str(len(value)) + ';')
            for key in value:
                self.put(key)
                self.put(value[key])
            return
        if isinstance(value, set):
            self.exactly(value, type(value) is set)
            if self.held(value):
                return
            self.pieces.append('S' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        if isinstance(value, frozenset):
            # A frozen set is a kind of its own and comes back as one.
            self.exactly(value, type(value) is frozenset)
            if self.held(value):
                return
            self.pieces.append('>' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        parts = _slice_parts(value)
        if parts is not None and self.slices:
            # A slice takes no place among the things held: it cannot
            # hold itself, and this runtime gives a slice no id to keep
            # a place by, so one standing twice is written twice.
            self.pieces.append('Q')
            self.put(parts[0])
            self.put(parts[1])
            self.put(parts[2])
            return
        # A value this runtime keeps no built-in writing for, such as
        # an `enumerate` or a `reversed`, is asked how it reduces: the
        # pieces that build it again, standing at the very place this
        # one does. Nothing else here holds itself, so this takes no
        # place among the things held either.
        reduced = __reduce_native__(value)
        if reduced is not None:
            self.pieces.append('Z')
            self.put(reduced)
            return
        self.refuse(value)

    def exactly(self, value, plain):
        # A class of its own built on a built-in kind is refused rather
        # than written as the kind beneath it, which would hand back
        # something of the wrong class.
        if not plain:
            self.refuse(value)


class _Reader:
    def __init__(self, text):
        self.text = text
        self.at = 0
        self.memo = []

    def word(self):
        start = self.at
        while self.at < len(self.text) and self.text[self.at] != ';':
            self.at += 1
        if self.at >= len(self.text):
            raise ValueError('bad marshal data')
        piece = self.text[start:self.at]
        self.at += 1
        return piece

    def count(self):
        return int(self.word())

    def characters(self, size):
        if self.at + size > len(self.text):
            raise ValueError('bad marshal data')
        piece = self.text[self.at:self.at + size]
        self.at += size
        return piece

    def reserve(self):
        where = len(self.memo)
        self.memo.append(None)
        return where

    def items(self, size):
        made = []
        done = 0
        while done < size:
            made.append(self.get())
            done += 1
        return made

    def get(self):
        if self.at >= len(self.text):
            raise ValueError('bad marshal data')
        tag = self.text[self.at]
        self.at += 1
        if tag == 'n':
            return None
        if tag == 'T':
            return True
        if tag == 'F':
            return False
        if tag == 'i':
            return int(self.word())
        if tag == 'r':
            return float(self.word())
        if tag == 'x':
            real = float(self.word())
            imaginary = float(self.word())
            return complex(real, imaginary)
        if tag == 'u':
            return self.characters(self.count())
        if tag == 'y':
            size = self.count()
            return bytes.fromhex(self.characters(size * 2))
        if tag == 'b':
            where = self.count()
            if where >= len(self.memo):
                raise ValueError('bad marshal data')
            standing = self.memo[where]
            if standing is None:
                raise ValueError('a loop through a tuple or a set cannot be read back')
            return standing
        if tag == 'L':
            size = self.count()
            made = []
            self.memo.append(made)
            done = 0
            while done < size:
                made.append(self.get())
                done += 1
            return made
        if tag == 'D':
            size = self.count()
            made = {}
            self.memo.append(made)
            done = 0
            while done < size:
                key = self.get()
                made[key] = self.get()
                done += 1
            return made
        if tag == 'P':
            size = self.count()
            where = self.reserve()
            made = tuple(self.items(size))
            self.memo[where] = made
            return made
        if tag == 'S':
            size = self.count()
            where = self.reserve()
            made = set(self.items(size))
            self.memo[where] = made
            return made
        if tag == '>':
            size = self.count()
            where = self.reserve()
            made = frozenset(self.items(size))
            self.memo[where] = made
            return made
        if tag == 'Q':
            start = self.get()
            stop = self.get()
            step = self.get()
            return slice(start, stop, step)
        if tag == 'Z':
            return __rebuild_native__(self.get())
        raise ValueError('bad marshal data')


def _slice_parts(value):
    # A slice is the one shape here that type() cannot name, so it is
    # asked for its own parts instead. Anything that will not answer for
    # all four is not a slice, whatever it says in refusing.
    try:
        start = value.start
        stop = value.stop
        step = value.step
        cut = value.indices
    except Exception:
        return None
    if not callable(cut):
        return None
    return (start, stop, step)


def _write(value, slices, refuse):
    writer = _Writer(slices, refuse)
    writer.put(value)
    return writer.text().encode()


def _read(data):
    if isinstance(data, str):
        raise TypeError('a str is not written-out bytes')
    if not data:
        raise EOFError('EOF read where object expected')
    return _Reader(data.decode()).get()


def _unwritable(value):
    raise ValueError('unmarshallable object')


# What is written and read by dumps, loads, dump and load below is the
# reference implementation's marshal format (Python/marshal.c), so data
# the reference wrote, such as the statistics file pstats reads, is read
# here and data written here is read by the reference. The writing above
# stays the private envelope that pickle builds on.
#
# Differences from the reference writer, none of which changes what
# comes back: only objects that stand more than once in the data are
# marked for later reference (the reference also marks small integers and
# interned strings, whose reference counts this runtime does not keep),
# and no string is written as interned. Code objects are not written.

version = 5

_FLAG_REF = 0x80
_MAX_DEPTH = 2000


def _unmarshallable():
    raise ValueError('unmarshallable object')


def _int32(number):
    return (number & 0xffffffff).to_bytes(4, 'little')


class _Counter:
    # A first walk over the data finding the objects that stand twice.
    def __init__(self):
        self.seen = {}
        self.kept = []

    def walk(self, value, depth):
        if depth > _MAX_DEPTH:
            raise ValueError('object too deeply nested to marshal')
        kind = type(value)
        if kind is tuple or kind is list or kind is dict or kind is set or kind is frozenset \
                or kind is str or kind is bytes:
            where = id(value)
            if where in self.seen:
                self.seen[where] += 1
                return
            self.seen[where] = 1
            self.kept.append(value)
        else:
            return
        if kind is tuple or kind is list or kind is set or kind is frozenset:
            for item in value:
                self.walk(item, depth + 1)
        elif kind is dict:
            for key in value:
                self.walk(key, depth + 1)
                self.walk(value[key], depth + 1)


class _Out:
    def __init__(self, edition):
        self.edition = edition
        self.parts = []
        self.places = {}
        self.counts = {}
        self.kept = []

    def result(self):
        return b''.join(self.parts)

    def mark(self, value):
        # True when the object was written before and is now written as
        # a reference to it; otherwise its type byte gets the flag if it
        # will be wanted again, and the caller writes the object.
        if self.edition < 3:
            return 0
        where = id(value)
        if self.counts.get(where, 0) < 2:
            return 0
        if where in self.places:
            self.parts.append(b'r' + _int32(self.places[where]))
            return -1
        self.places[where] = len(self.places)
        return _FLAG_REF

    def put(self, value, depth):
        parts = self.parts
        if depth > _MAX_DEPTH:
            raise ValueError('object too deeply nested to marshal')
        if value is None:
            parts.append(b'N')
            return
        if value is True:
            parts.append(b'T')
            return
        if value is False:
            parts.append(b'F')
            return
        if value is Ellipsis:
            parts.append(b'.')
            return
        if value is StopIteration:
            parts.append(b'S')
            return
        kind = type(value)
        if kind is int:
            if -2147483648 <= value <= 2147483647:
                parts.append(b'i' + _int32(value))
                return
            self.put_long(value)
            return
        if kind is float:
            if self.edition > 1:
                parts.append(b'g' + struct.pack('<d', value))
            else:
                text = repr(value).encode('ascii')
                parts.append(b'f' + bytes([len(text)]) + text)
            return
        if kind is complex:
            if self.edition > 1:
                parts.append(b'y' + struct.pack('<d', value.real) + struct.pack('<d', value.imag))
            else:
                one = repr(value.real).encode('ascii')
                two = repr(value.imag).encode('ascii')
                parts.append(b'x' + bytes([len(one)]) + one + bytes([len(two)]) + two)
            return
        if kind is str:
            flag = self.mark(value)
            if flag < 0:
                return
            try:
                data = value.encode('utf-8', 'surrogatepass')
            except Exception:
                _unmarshallable()
            if self.edition >= 4 and value.isascii():
                if len(data) < 256:
                    parts.append(bytes([ord('z') | flag, len(data)]) + data)
                else:
                    parts.append(bytes([ord('a') | flag]) + _int32(len(data)) + data)
            else:
                parts.append(bytes([ord('u') | flag]) + _int32(len(data)) + data)
            return
        if kind is bytes or kind is bytearray or kind is memoryview:
            flag = self.mark(value) if kind is bytes else 0
            if flag < 0:
                return
            data = bytes(value)
            parts.append(bytes([ord('s') | flag]) + _int32(len(data)) + data)
            return
        if kind is tuple:
            flag = self.mark(value)
            if flag < 0:
                return
            if self.edition >= 4 and len(value) < 256:
                parts.append(bytes([ord(')') | flag, len(value)]))
            else:
                parts.append(bytes([ord('(') | flag]) + _int32(len(value)))
            for item in value:
                self.put(item, depth + 1)
            return
        if kind is list:
            flag = self.mark(value)
            if flag < 0:
                return
            parts.append(bytes([ord('[') | flag]) + _int32(len(value)))
            for item in value:
                self.put(item, depth + 1)
            return
        if kind is dict:
            flag = self.mark(value)
            if flag < 0:
                return
            parts.append(bytes([ord('{') | flag]))
            for key in value:
                self.put(key, depth + 1)
                self.put(value[key], depth + 1)
            parts.append(b'0')
            return
        if kind is set or kind is frozenset:
            flag = self.mark(value)
            if flag < 0:
                return
            parts.append(bytes([ord('<' if kind is set else '>') | flag]) + _int32(len(value)))
            for item in value:
                self.put(item, depth + 1)
            return
        if kind is slice and self.edition >= 5:
            parts.append(b':')
            self.put(value.start, depth + 1)
            self.put(value.stop, depth + 1)
            self.put(value.step, depth + 1)
            return
        _unmarshallable()

    def put_long(self, number):
        negative = number < 0
        rest = -number if negative else number
        digits = []
        while rest:
            digits.append(rest & 0x7fff)
            rest >>= 15
        size = len(digits)
        self.parts.append(b'l' + _int32(-size if negative else size)
                          + b''.join(digit.to_bytes(2, 'little') for digit in digits))


def dumps(value, version=version, /, *, allow_code=True):
    """Return the bytes object that would be written to a file by dump(value, file).

    The value must be a supported type.  Raise a ValueError exception if
    value has (or contains an object that has) an unsupported type.
    """
    if not isinstance(version, int):
        raise TypeError("'" + type(version).__name__ + "' object cannot be interpreted as an integer")
    out = _Out(version)
    if version >= 3:
        counter = _Counter()
        counter.walk(value, 0)
        out.counts = counter.seen
        out.kept = counter.kept
    out.put(value, 0)
    return out.result()


def dump(value, file, version=version, /, *, allow_code=True):
    """Write the value on the open file.

    The value must be a supported type.  The file must be a writeable
    binary file.
    """
    file.write(dumps(value, version))


class _In:
    # `buf` holds the bytes taken so far and `at` the place read up to;
    # a reader over a file takes from it only what is wanted, so that
    # the file stands just after the object that was read.
    def __init__(self, buf, source):
        self.buf = buf
        self.at = 0
        self.source = source
        self.refs = []
        self.interned = []

    def need(self, size):
        end = self.at + size
        if end > len(self.buf):
            if self.source is not None:
                more = self.source(end - len(self.buf))
                if more:
                    self.buf = self.buf + bytes(more)
            if end > len(self.buf):
                if self.at == 0 and size == 1:
                    raise EOFError('EOF read where object expected')
                raise EOFError('marshal data too short')

    def read(self, size):
        self.need(size)
        start = self.at
        self.at = start + size
        return self.buf[start:self.at]

    def byte(self):
        self.need(1)
        self.at += 1
        return self.buf[self.at - 1]

    def int32(self):
        return int.from_bytes(self.read(4), 'little', signed=True)

    def size32(self):
        count = self.int32()
        if count < 0:
            raise ValueError('bad marshal data (size out of range)')
        return count

    def text_float(self):
        size = self.byte()
        return float(self.read(size).decode('ascii'))

    def get(self, depth, closing=False):
        if depth > _MAX_DEPTH:
            raise ValueError('recursion limit exceeded')
        code = self.byte()
        flag = code & _FLAG_REF
        tag = chr(code & 0x7f)
        refs = self.refs
        if tag == '0':
            if closing:
                return _UNSET
            raise TypeError('NULL object in marshal data for object')
        if tag == 'N':
            return None
        if tag == 'T':
            return True
        if tag == 'F':
            return False
        if tag == '.':
            return Ellipsis
        if tag == 'S':
            return StopIteration
        if tag == 'r':
            index = self.int32()
            if index < 0 or index >= len(refs):
                raise ValueError('bad marshal data (invalid reference)')
            if refs[index] is _UNSET:
                raise ValueError('bad marshal data (invalid reference)')
            return refs[index]
        if tag == '(' or tag == '[' or tag == '<' or tag == '>':
            return self.sequence(tag, self.size32(), flag, depth)
        if tag == ')':
            return self.sequence('(', self.byte(), flag, depth)
        if tag == '{':
            place = self.reserve(flag)
            made = {}
            if place >= 0:
                refs[place] = made
            while True:
                key = self.get(depth + 1, True)
                if key is _UNSET:
                    break
                made[key] = self.get(depth + 1)
            return made
        if tag == ':':
            place = self.reserve(flag)
            start = self.get(depth + 1)
            stop = self.get(depth + 1)
            step = self.get(depth + 1)
            made = slice(start, stop, step)
            if place >= 0:
                refs[place] = made
            return made
        if tag == 'i':
            made = self.int32()
        elif tag == 'I':
            made = int.from_bytes(self.read(8), 'little', signed=True)
        elif tag == 'l':
            count = self.int32()
            size = -count if count < 0 else count
            number = 0
            shift = 0
            for _ in range(size):
                digit = int.from_bytes(self.read(2), 'little')
                if digit > 0x7fff:
                    raise ValueError('bad marshal data (digit out of range in long)')
                number |= digit << shift
                shift += 15
            if size and number >> (shift - 15) == 0:
                raise ValueError('bad marshal data (unnormalized long data)')
            made = -number if count < 0 else number
        elif tag == 'f':
            made = self.text_float()
        elif tag == 'g':
            made = struct.unpack('<d', self.read(8))[0]
        elif tag == 'x':
            real = self.text_float()
            made = complex(real, self.text_float())
        elif tag == 'y':
            real = struct.unpack('<d', self.read(8))[0]
            made = complex(real, struct.unpack('<d', self.read(8))[0])
        elif tag == 's':
            made = self.read(self.size32())
        elif tag == 'u' or tag == 't':
            made = self.read(self.size32()).decode('utf-8', 'surrogatepass')
            if tag == 't':
                self.interned.append(made)
        elif tag == 'a' or tag == 'A':
            made = self.read(self.size32()).decode('ascii')
            if tag == 'A':
                self.interned.append(made)
        elif tag == 'z' or tag == 'Z':
            made = self.read(self.byte()).decode('ascii')
            if tag == 'Z':
                self.interned.append(made)
        elif tag == 'R':
            index = self.int32()
            if index < 0 or index >= len(self.interned):
                raise ValueError('bad marshal data (string ref out of range)')
            return self.interned[index]
        elif tag == 'c':
            raise ValueError('bad marshal data (code objects are not supported)')
        else:
            raise ValueError('bad marshal data (unknown type code)')
        if flag:
            refs.append(made)
        return made

    def reserve(self, flag):
        if not flag:
            return -1
        self.refs.append(_UNSET)
        return len(self.refs) - 1

    def sequence(self, tag, count, flag, depth):
        place = self.reserve(flag)
        if tag == '[':
            made = []
            if place >= 0:
                self.refs[place] = made
            for _ in range(count):
                made.append(self.get(depth + 1))
            return made
        items = [self.get(depth + 1) for _ in range(count)]
        if tag == '(':
            made = tuple(items)
        elif tag == '<':
            made = set(items)
        else:
            made = frozenset(items)
        if place >= 0:
            self.refs[place] = made
        return made


_UNSET = object()


def _bytes_of(data):
    if isinstance(data, str) or not isinstance(data, (bytes, bytearray, memoryview)):
        raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'")
    return bytes(data)


def loads(data, /, *, allow_code=True):
    """Convert the bytes-like object to a value.

    If no valid value is found, raise EOFError, ValueError or TypeError.
    Extra bytes in the input are ignored.
    """
    return _In(_bytes_of(data), None).get(0)


def load(file, /, *, allow_code=True):
    """Read one value from the open file and return it.

    If no valid value is read (e.g. because the data has a different Python
    version's incompatible marshal format), raise EOFError, ValueError or
    TypeError.

    Note: If an object containing an unsupported type was marshalled with
    dump(), load() will substitute None for the unmarshallable type.
    """
    return _In(b'', file.read).get(0)


def __getattr__(name):
    raise AttributeError("module 'marshal' has no attribute '" + name + "'")
