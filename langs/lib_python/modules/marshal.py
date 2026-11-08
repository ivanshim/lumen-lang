"""Marshal binary values with versions 0 through 5 and shared references.

Built-in data uses CPython's type tags and binary layouts. Code records
use this interpreter's immutable compilation input and complete constant
state; they are private to this runtime and are not CPython bytecode.
Compilation input is stored once per stream. Both directions honor
allow_code=False. No source file is reopened.
"""

import struct
import sys
import types
import io

version = 5

_MAX_DEPTH = 2000
_READ_STEP = 131072
_SIZE32_MAX = 0x7FFFFFFF
_FLAG_REF = 0x80
_code_prefix = b"\x00LumenCode\x01"

_TYPE_NULL = 0x30
_TYPE_NONE = 0x4E
_TYPE_FALSE = 0x46
_TYPE_TRUE = 0x54
_TYPE_STOPITER = 0x53
_TYPE_ELLIPSIS = 0x2E
_TYPE_INT = 0x69
_TYPE_INT64 = 0x49
_TYPE_FLOAT = 0x66
_TYPE_BINARY_FLOAT = 0x67
_TYPE_COMPLEX = 0x78
_TYPE_BINARY_COMPLEX = 0x79
_TYPE_LONG = 0x6C
_TYPE_STRING = 0x73
_TYPE_INTERNED = 0x74
_TYPE_REF = 0x72
_TYPE_TUPLE = 0x28
_TYPE_SMALL_TUPLE = 0x29
_TYPE_LIST = 0x5B
_TYPE_DICT = 0x7B
_TYPE_CODE = 0x63
_TYPE_UNICODE = 0x75
_TYPE_SET = 0x3C
_TYPE_FROZENSET = 0x3E
_TYPE_SLICE = 0x3A
_TYPE_ASCII = 0x61
_TYPE_ASCII_INTERNED = 0x41
_TYPE_SHORT_ASCII = 0x7A
_TYPE_SHORT_ASCII_INTERNED = 0x5A


def _pack_long(value):
    return (value & 0xFFFFFFFF).to_bytes(4, "little")


def _pack_short(value):
    return (value & 0xFFFF).to_bytes(2, "little")


def _pack_size(value):
    if value > _SIZE32_MAX:
        raise ValueError("unmarshallable object")
    return _pack_long(value)


def _pack_double(value):
    return struct.pack("<d", value)


def _float_text(value):
    # The reference lays a float out with seventeen significant figures
    # in the shortest spelling that keeps them; `%.17g` is that spelling
    # at every magnitude, and reads back to the very same float.
    return ("%.17g" % value).encode("ascii")


def _long_digits(value):
    negative = value < 0
    remaining = -value if negative else value
    digits = []
    while remaining:
        digits.append(remaining & 0x7FFF)
        remaining >>= 15
    if not digits:
        digits.append(0)
    size = len(digits)
    if negative:
        size = -size
    written = bytearray(_pack_long(size))
    for digit in digits:
        written.extend(_pack_short(digit))
    return written


def _is_ascii(text):
    return text.isascii()


def _is_interned(text):
    if len(text) >= 256:
        return False
    try:
        return sys.intern(text) is text
    except Exception:
        return False


_MODE_NAMES = ("exec", "eval", "single")


def _fresh_whole(value):
    """A number of its own: the reference's unmarshalling makes a new
    whole number for each occurrence outside its small cache, and one
    shared number inside it."""
    return __trace_native__(13, value)


# Capture a live code object's reconstruction input and full constant state.
def _code_pieces(value):
    source, boundary, private, features, optimize, lineless = __trace_native__(14, value)
    constants = []
    for at, const in enumerate(value.co_consts):
        constants.append(None if at in private else const)
    return (boundary, source, value.co_filename, getattr(value, "mode", 0),
            value.co_flags, value.co_firstlineno,
            getattr(value, "co_name", ""), getattr(value, "co_qualname", ""),
            tuple(constants), private, optimize, features, lineless)


# Index compiled bodies once before applying mutable code metadata.
def _code_index(block):
    bodies = {}
    pending = [block]
    while pending:
        parent = pending.pop()
        for const in getattr(parent, "co_consts", ()) or ():
            if isinstance(const, types.CodeType):
                locator = __trace_native__(14, const)[1]
                if locator not in bodies:
                    bodies[locator] = const
                    pending.append(const)
    return bodies


class _Pending:
    """The place a tuple takes before its contents are known.

    A tuple has no empty form that can be filled in, so while its
    elements are being read the reference table keeps this marker; a
    reference meeting it gets the marker and the real tuple is put in
    its place, and in every place the marker reached, once the elements
    are all in.
    """

    __slots__ = ("locations",)

    def __init__(self):
        self.locations = []


class _Writer:
    def __init__(self, written_version, allow_code):
        self.version = written_version
        self.allow_code = allow_code
        self.memo = {}
        self.kept = []
        self.pending = set()
        self.out = bytearray()
        self.sources = {}
        self.depth = 0

    def write(self, value):
        stack = [("value", value)]
        while stack:
            action, payload = stack.pop()
            if action == "pop":
                self.depth -= 1
                continue
            if action == "raw":
                self.out.extend(payload)
                continue
            if action == "unpend":
                self.pending.discard(payload)
                continue
            self.depth += 1
            if self.depth > _MAX_DEPTH:
                raise ValueError("object too deeply nested to marshal")
            self._one(payload, stack)

    def _one(self, value, stack):
        out = self.out
        written_version = self.version
        if value is None:
            out.append(_TYPE_NONE)
            self.depth -= 1
            return
        if value is True:
            out.append(_TYPE_TRUE)
            self.depth -= 1
            return
        if value is False:
            out.append(_TYPE_FALSE)
            self.depth -= 1
            return
        if value is StopIteration:
            out.append(_TYPE_STOPITER)
            self.depth -= 1
            return
        if value is Ellipsis:
            out.append(_TYPE_ELLIPSIS)
            self.depth -= 1
            return

        flag = 0
        if written_version >= 3:
            where = self.memo.get(id(value))
            if where is not None:
                if id(value) in self.pending:
                    raise ValueError(
                        "cannot marshal recursion in %s objects"
                        % type(value).__name__
                    )
                out.append(_TYPE_REF)
                out.extend(_pack_long(where))
                self.depth -= 1
                return
            self.memo[id(value)] = len(self.kept)
            self.kept.append(value)
            flag = _FLAG_REF

        kind = type(value)
        if kind is int:
            if -0x80000000 <= value <= 0x7FFFFFFF:
                out.append(_TYPE_INT | flag)
                out.extend(_pack_long(value))
            else:
                out.append(_TYPE_LONG | flag)
                out.extend(_long_digits(value))
            self.depth -= 1
        elif kind is float:
            if written_version > 1:
                out.append(_TYPE_BINARY_FLOAT | flag)
                out.extend(_pack_double(value))
            else:
                out.append(_TYPE_FLOAT | flag)
                self._string_piece(_float_text(value))
            self.depth -= 1
        elif kind is complex:
            if written_version > 1:
                out.append(_TYPE_BINARY_COMPLEX | flag)
                out.extend(_pack_double(value.real))
                out.extend(_pack_double(value.imag))
            else:
                out.append(_TYPE_COMPLEX | flag)
                self._string_piece(_float_text(value.real))
                self._string_piece(_float_text(value.imag))
            self.depth -= 1
        elif kind is bytes:
            out.append(_TYPE_STRING | flag)
            out.extend(_pack_size(len(value)))
            out.extend(value)
            self.depth -= 1
        elif kind is str:
            self._write_text(value, flag)
            self.depth -= 1
        elif kind is tuple:
            count = len(value)
            if written_version >= 4 and count < 256:
                out.append(_TYPE_SMALL_TUPLE | flag)
                out.append(count)
            else:
                out.append(_TYPE_TUPLE | flag)
                out.extend(_pack_size(count))
            stack.append(("pop", None))
            for index in range(count - 1, -1, -1):
                stack.append(("value", value[index]))
        elif kind is list:
            out.append(_TYPE_LIST | flag)
            out.extend(_pack_size(len(value)))
            stack.append(("pop", None))
            for index in range(len(value) - 1, -1, -1):
                stack.append(("value", value[index]))
        elif kind is dict:
            out.append(_TYPE_DICT | flag)
            stack.append(("pop", None))
            stack.append(("raw", bytes([_TYPE_NULL])))
            items = list(value.items())
            for index in range(len(items) - 1, -1, -1):
                key, item = items[index]
                stack.append(("value", item))
                stack.append(("value", key))
        elif kind is set or kind is frozenset:
            out.append((_TYPE_SET if kind is set else _TYPE_FROZENSET) | flag)
            out.extend(_pack_size(len(value)))
            order = sorted(value, key=self._sort_key)
            stack.append(("pop", None))
            for index in range(len(order) - 1, -1, -1):
                stack.append(("value", order[index]))
        elif isinstance(value, types.CodeType):
            if not self.allow_code:
                raise ValueError("marshalling code objects is disallowed")
            out.append(_TYPE_CODE | flag)
            parts = _code_pieces(value)
            source = parts[1]
            unit = self.sources.get(source)
            if unit is None:
                unit = len(self.sources)
                self.sources[source] = unit
                out.extend(_pack_size(unit))
                encoded = source.encode("utf-8", "surrogatepass")
                out.extend(_pack_size(len(encoded)))
                out.extend(encoded)
            else:
                out.extend(_pack_size(unit))
            self.pending.add(id(value))
            stack.append(("unpend", id(value)))
            stack.append(("pop", None))
            stack.append(("value", parts[:1] + parts[2:]))
        elif kind is slice:
            if written_version < 5:
                raise ValueError("unmarshallable object")
            out.append(_TYPE_SLICE | flag)
            self.pending.add(id(value))
            stack.append(("unpend", id(value)))
            stack.append(("pop", None))
            stack.append(("value", value.step))
            stack.append(("value", value.stop))
            stack.append(("value", value.start))
        else:
            try:
                view = memoryview(value)
            except TypeError:
                raise ValueError("unmarshallable object")
            data = view.tobytes()
            out.append(_TYPE_STRING | flag)
            out.extend(_pack_size(len(data)))
            out.extend(data)
            self.depth -= 1

    def _string_piece(self, data):
        self.out.append(len(data))
        self.out.extend(data)

    def _sort_key(self, element):
        fresh = _Writer(self.version, self.allow_code)
        fresh.write(element)
        return bytes(fresh.out)

    def _write_text(self, value, flag):
        out = self.out
        interned = _is_interned(value)
        if self.version >= 4 and _is_ascii(value):
            text = value.encode("ascii")
            if len(value) < 256:
                out.append(
                    (_TYPE_SHORT_ASCII_INTERNED if interned else _TYPE_SHORT_ASCII)
                    | flag
                )
                out.append(len(value))
            else:
                out.append(
                    (_TYPE_ASCII_INTERNED if interned else _TYPE_ASCII) | flag
                )
                out.extend(_pack_size(len(value)))
            out.extend(text)
            return
        try:
            text = value.encode("utf-8", "surrogatepass")
        except Exception:
            try:
                text = value.encode("utf-8")
            except Exception:
                raise ValueError("unmarshallable object")
        if self.version >= 3 and interned:
            out.append(_TYPE_INTERNED | flag)
        else:
            out.append(_TYPE_UNICODE | flag)
        out.extend(_pack_size(len(text)))
        out.extend(text)


class _ListFrame:
    __slots__ = ("box", "left", "result")

    def __init__(self, box, count):
        self.box = box
        self.left = count
        self.result = box

    def feed(self, value):
        if value is _NULL:
            raise TypeError("NULL object in marshal data for list")
        self.box.append(value)
        if isinstance(value, _Pending):
            value.locations.append((self.box, len(self.box) - 1))
        self.left -= 1
        return self.left == 0


class _TupleFrame:
    __slots__ = ("memo", "index", "pending", "collected", "left", "result")

    def __init__(self, memo, index, pending, count):
        self.memo = memo
        self.index = index
        self.pending = pending
        self.collected = []
        self.left = count
        self.result = None

    def feed(self, value):
        if value is _NULL:
            raise TypeError("NULL object in marshal data for tuple")
        self.collected.append(value)
        self.left -= 1
        if self.left:
            return False
        for element in self.collected:
            if element is self.pending:
                raise ValueError("bad marshal data (recursive tuple)")
        made = tuple(self.collected)
        for box, key in self.pending.locations:
            box[key] = made
        if self.index is not None:
            self.memo[self.index] = made
        self.result = made
        return True


class _DictFrame:
    __slots__ = ("box", "expecting_key", "key", "result")

    def __init__(self, box):
        self.box = box
        self.expecting_key = True
        self.key = None
        self.result = box

    def feed(self, value):
        if self.expecting_key:
            if value is _NULL:
                self.result = self.box
                return True
            if isinstance(value, _Pending):
                raise ValueError("bad marshal data (recursive dict key)")
            self.key = value
            self.expecting_key = False
            return False
        if value is _NULL:
            self.result = self.box
            return True
        self.box[self.key] = value
        if isinstance(value, _Pending):
            value.locations.append((self.box, self.key))
        self.expecting_key = True
        return False


class _SetFrame:
    __slots__ = ("box", "left", "result")

    def __init__(self, box, count):
        self.box = box
        self.left = count
        self.result = box

    def feed(self, value):
        if value is _NULL:
            raise TypeError("NULL object in marshal data for set")
        if isinstance(value, _Pending):
            raise ValueError("bad marshal data (recursive set)")
        self.box.add(value)
        self.left -= 1
        return self.left == 0


class _FrozenFrame:
    __slots__ = ("memo", "index", "collected", "left", "result")

    def __init__(self, memo, index, count):
        self.memo = memo
        self.index = index
        self.collected = []
        self.left = count
        self.result = None

    def feed(self, value):
        if value is _NULL:
            raise TypeError("NULL object in marshal data for set")
        if isinstance(value, _Pending):
            raise ValueError("bad marshal data (recursive set)")
        self.collected.append(value)
        self.left -= 1
        if self.left:
            return False
        made = frozenset(self.collected)
        if self.index is not None:
            self.memo[self.index] = made
        self.result = made
        return True


class _SliceFrame:
    __slots__ = ("memo", "index", "parts", "left", "result")

    def __init__(self, memo, index):
        self.memo = memo
        self.index = index
        self.parts = []
        self.left = 3
        self.result = None

    def feed(self, value):
        if value is _NULL:
            raise ValueError("bad marshal data (NULL in slice)")
        if isinstance(value, _Pending):
            raise ValueError("bad marshal data (invalid reference)")
        self.parts.append(value)
        self.left -= 1
        if self.left:
            return False
        made = slice(self.parts[0], self.parts[1], self.parts[2])
        if self.index is not None:
            self.memo[self.index] = made
        self.result = made
        return True


class _CodeFrame:
    __slots__ = ("memo", "index", "codes", "source", "result")

    def __init__(self, memo, index, codes, source):
        self.memo = memo
        self.index = index
        self.codes = codes
        self.source = source
        self.result = None

    # Rebuild the retained body, then restore every serialized constant and flag.
    def feed(self, value):
        if not isinstance(value, tuple) or len(value) != 12:
            raise ValueError("bad marshal data (code object)")
        boundary, filename, mode, flags, firstlineno, name, qualname, consts, private, optimize, features, lineless = value
        source = self.source
        if (mode not in (0, 1, 2) or not isinstance(source, str)
                or not isinstance(consts, tuple) or not isinstance(private, tuple)
                or not isinstance(lineless, bool)):
            raise ValueError("bad marshal data (code object)")
        compile_flags = features
        key = (source, filename, mode, optimize, compile_flags)
        compiled = self.codes.get(key)
        if compiled is None:
            block = compile(source, filename, _MODE_NAMES[mode], flags=compile_flags,
                            dont_inherit=True, optimize=optimize)
            compiled = (block, _code_index(block))
            self.codes[key] = compiled
        block, bodies = compiled
        result = block if boundary == -1 else bodies.get(boundary)
        if result is None:
            raise ValueError("bad marshal data (code object)")
        restored = list(consts)
        for at in private:
            if (not isinstance(at, int) or at < 0 or at >= len(restored)
                    or at >= len(result.co_consts) or restored[at] is not None
                    or not isinstance(result.co_consts[at], type)):
                raise ValueError("bad marshal data (code object)")
            restored[at] = result.co_consts[at]
        if boundary == -1:
            result = result.replace(co_consts=tuple(restored), co_flags=flags)
        else:
            result = result.replace(co_consts=tuple(restored), co_flags=flags,
                                    co_name=name, co_qualname=qualname,
                                    co_firstlineno=firstlineno)
        if lineless:
            result = result.replace(co_linetable=b"")
        if self.index is not None:
            self.memo[self.index] = result
        self.result = result
        return True


_NULL = object()
_NO_RESULT = object()


class _Reader:
    def __init__(self, data, file, allow_code):
        self.data = data
        self.file = file
        # Native binary streams already return exact bytes; subclasses keep readinto validation.
        self.read_bytes = file.read if type(file) in (io.FileIO, io.BufferedReader, io.BufferedRandom, io.BytesIO) else None
        self.allow_code = allow_code
        self.at = 0
        self.memo = []
        self.frames = []
        self.codes = {}
        self.sources = []
        self.depth = 0

    def _bytes(self, count):
        if self.data is not None:
            end = self.at + count
            if end > len(self.data):
                raise EOFError("marshal data too short")
            piece = self.data[self.at:end]
            self.at = end
            return piece
        if count == 0:
            return b""
        if self.read_bytes is not None:
            piece = self.read_bytes(count)
            if len(piece) != count:
                raise EOFError("EOF read where not expected")
            return piece
        buffer = bytearray(count)
        try:
            before = self.file.tell()
        except (AttributeError, OSError):
            before = None
        if not hasattr(self.file, "readinto"):
            piece = self.file.read(count)
            if not isinstance(piece, bytes):
                raise TypeError("file.read() returned not bytes but " + type(piece).__name__)
            if len(piece) > count:
                raise ValueError("read() returned too much data: %d bytes requested, %d returned" % (count, len(piece)))
            if len(piece) != count:
                raise EOFError("EOF read where not expected")
            return piece
        try:
            read = self.file.readinto(memoryview(buffer))
        except ValueError as fault:
            # A buffered stream handed a target that only part of one
            # refill can fill may copy more than the target can take,
            # though the bytes it was to copy are all there.  What it
            # did copy stands at the front of the target and the stream
            # now stands just past it, so the rest is taken through
            # read, which hands back what was asked for and no more: a
            # load still leaves the stream where the object it read
            # ended.
            if before is None or "memoryview" not in str(fault):
                raise
            copied = self.file.tell() - before
            if copied < 0 or copied > count:
                raise
            rest = self.file.read(count - copied)
            if rest is None or len(rest) != count - copied:
                raise EOFError("EOF read where not expected")
            return bytes(buffer[:copied]) + bytes(rest)
        if read is None:
            raise EOFError("EOF read where not expected")
        if read > count:
            raise ValueError(
                "read() returned too much data: %d bytes requested, %d returned"
                % (count, read)
            )
        if read != count:
            raise EOFError("EOF read where not expected")
        return bytes(buffer)

    def _byte(self):
        if self.data is not None:
            if self.at >= len(self.data):
                raise EOFError("EOF read where object expected")
            value = self.data[self.at]
            self.at += 1
            return value
        piece = self._bytes(1)
        return piece[0]

    def _long(self):
        raw = self._bytes(4)
        value = raw[0] | (raw[1] << 8) | (raw[2] << 16) | (raw[3] << 24)
        if value & 0x80000000:
            value -= 0x100000000
        return value

    def _short(self):
        raw = self._bytes(2)
        value = raw[0] | (raw[1] << 8)
        if value & 0x8000:
            value -= 0x10000
        return value

    def _pylong(self):
        size = self._long()
        if size == 0:
            return 0
        negative = size < 0
        digits = -size if negative else size
        if digits > _SIZE32_MAX:
            raise ValueError("bad marshal data (long size out of range)")
        value = 0
        shift = 0
        for index in range(digits):
            digit = self._short()
            if digit < 0 or digit > 0x8000:
                raise ValueError("bad marshal data (digit out of range in long)")
            if index == digits - 1 and digit == 0:
                raise ValueError("bad marshal data (unnormalized long data)")
            value |= digit << shift
            shift += 15
        return -value if negative else value

    def _float_text(self):
        count = self._byte()
        return float(self._bytes(count).decode("latin-1"))

    def _ref(self, flag, value):
        if flag:
            self.memo.append(value)

    def _step(self):
        code = self._byte()
        tag = code & 0x7F
        flag = code & _FLAG_REF
        self.depth += 1
        if self.depth > _MAX_DEPTH:
            self.depth -= 1
            raise ValueError("recursion limit exceeded")

        if tag == 0 and not flag and self.file is not None and not self.frames:
            try:
                prefix = self._bytes(len(_code_prefix) - 1)
            except EOFError:
                raise ValueError("bad marshal data (unknown type code)")
            if prefix != _code_prefix[1:]:
                raise ValueError("bad marshal data (unknown type code)")
            if not self.allow_code:
                raise ValueError("unmarshalling code objects is disallowed")
            # Legacy native cache envelopes occupy the remainder of the stream.
            value = _code_unpack(self.file.read())
            self.depth -= 1
            return value
        if tag == _TYPE_NULL:
            self.depth -= 1
            return _NULL
        if tag == _TYPE_NONE:
            self.depth -= 1
            return None
        if tag == _TYPE_TRUE:
            self.depth -= 1
            return True
        if tag == _TYPE_FALSE:
            self.depth -= 1
            return False
        if tag == _TYPE_STOPITER:
            self.depth -= 1
            return StopIteration
        if tag == _TYPE_ELLIPSIS:
            self.depth -= 1
            return Ellipsis
        if tag == _TYPE_INT:
            value = _fresh_whole(self._long())
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_INT64:
            value = _fresh_whole(int.from_bytes(self._bytes(8), "little", signed=True))
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_LONG:
            value = _fresh_whole(self._pylong())
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_FLOAT:
            value = self._float_text()
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_BINARY_FLOAT:
            value = struct.unpack("<d", self._bytes(8))[0]
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_COMPLEX:
            real = self._float_text()
            imaginary = self._float_text()
            value = complex(real, imaginary)
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_BINARY_COMPLEX:
            real = struct.unpack("<d", self._bytes(8))[0]
            imaginary = struct.unpack("<d", self._bytes(8))[0]
            value = complex(real, imaginary)
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_STRING:
            count = self._long()
            if count < 0 or count > _SIZE32_MAX:
                raise ValueError("bad marshal data (bytes object size out of range)")
            value = self._bytes(count)
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_SHORT_ASCII or tag == _TYPE_SHORT_ASCII_INTERNED:
            count = self._byte()
            value = self._bytes(count).decode("latin-1")
            if tag == _TYPE_SHORT_ASCII_INTERNED:
                value = sys.intern(value)
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_ASCII or tag == _TYPE_ASCII_INTERNED:
            count = self._long()
            if count < 0 or count > _SIZE32_MAX:
                raise ValueError("bad marshal data (string size out of range)")
            value = self._bytes(count).decode("latin-1")
            if tag == _TYPE_ASCII_INTERNED:
                value = sys.intern(value)
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_INTERNED or tag == _TYPE_UNICODE:
            count = self._long()
            if count < 0 or count > _SIZE32_MAX:
                raise ValueError("bad marshal data (string size out of range)")
            if count == 0:
                value = ""
            else:
                raw = self._bytes(count)
                value = raw.decode("utf-8", "surrogatepass")
            if tag == _TYPE_INTERNED:
                value = sys.intern(value)
            self._ref(flag, value)
            self.depth -= 1
            return value
        if tag == _TYPE_SMALL_TUPLE or tag == _TYPE_TUPLE:
            if tag == _TYPE_SMALL_TUPLE:
                count = self._byte()
            else:
                count = self._long()
                if count < 0 or count > _SIZE32_MAX:
                    raise ValueError("bad marshal data (tuple size out of range)")
            if count == 0:
                value = ()
                self._ref(flag, value)
                self.depth -= 1
                return value
            if flag:
                index = len(self.memo)
                pending = _Pending()
                self.memo.append(pending)
            else:
                index = None
                pending = _Pending()
            self.frames.append(_TupleFrame(self.memo, index, pending, count))
            return _NO_RESULT
        if tag == _TYPE_LIST:
            count = self._long()
            if count < 0 or count > _SIZE32_MAX:
                raise ValueError("bad marshal data (list size out of range)")
            box = []
            if flag:
                self.memo.append(box)
            if count == 0:
                self.depth -= 1
                return box
            self.frames.append(_ListFrame(box, count))
            return _NO_RESULT
        if tag == _TYPE_DICT:
            box = {}
            if flag:
                self.memo.append(box)
            self.frames.append(_DictFrame(box))
            return _NO_RESULT
        if tag == _TYPE_SET:
            count = self._long()
            if count < 0 or count > _SIZE32_MAX:
                raise ValueError("bad marshal data (set size out of range)")
            box = set()
            if flag:
                self.memo.append(box)
            if count == 0:
                self.depth -= 1
                return box
            self.frames.append(_SetFrame(box, count))
            return _NO_RESULT
        if tag == _TYPE_FROZENSET:
            count = self._long()
            if count < 0 or count > _SIZE32_MAX:
                raise ValueError("bad marshal data (set size out of range)")
            if count == 0:
                value = frozenset()
                self._ref(flag, value)
                self.depth -= 1
                return value
            if flag:
                index = len(self.memo)
                self.memo.append(None)
            else:
                index = None
            self.frames.append(_FrozenFrame(self.memo, index, count))
            return _NO_RESULT
        if tag == _TYPE_SLICE:
            if flag:
                index = len(self.memo)
                self.memo.append(None)
            else:
                index = None
            self.frames.append(_SliceFrame(self.memo, index))
            return _NO_RESULT
        if tag == _TYPE_CODE:
            if not self.allow_code:
                raise ValueError("unmarshalling code objects is disallowed")
            if flag:
                index = len(self.memo)
                self.memo.append(None)
            else:
                index = None
            unit = self._long()
            if unit == len(self.sources):
                size = self._long()
                if size < 0:
                    raise ValueError("bad marshal data (code source size out of range)")
                source = self._bytes(size).decode("utf-8", "surrogatepass")
                self.sources.append(source)
            elif 0 <= unit < len(self.sources):
                source = self.sources[unit]
            else:
                raise ValueError("bad marshal data (code source index out of range)")
            self.frames.append(_CodeFrame(self.memo, index, self.codes, source))
            return _NO_RESULT
        if tag == _TYPE_REF:
            index = self._long()
            if index < 0 or index >= len(self.memo):
                raise ValueError("bad marshal data (invalid reference)")
            value = self.memo[index]
            if value is None:
                raise ValueError("bad marshal data (invalid reference)")
            self.depth -= 1
            return value
        raise ValueError("bad marshal data (unknown type code)")

    def read(self):
        value = _NO_RESULT
        while True:
            if value is _NO_RESULT:
                value = self._step()
                if value is _NO_RESULT:
                    continue
            while self.frames:
                frame = self.frames[-1]
                if frame.feed(value):
                    self.frames.pop()
                    self.depth -= 1
                    value = frame.result
                    continue
                value = _NO_RESULT
                break
            else:
                if value is _NULL:
                    raise TypeError("NULL object in marshal data for object")
                return value


def dumps(value, version=version, /, *, allow_code=True):
    writer = _Writer(version, allow_code)
    writer.write(value)
    return bytes(writer.out)


def dump(value, file, version=version, /, *, allow_code=True):
    writer = _Writer(version, allow_code)
    writer.write(value)
    file.write(bytes(writer.out))


def loads(data, /, *, allow_code=True):
    if isinstance(data, str):
        raise TypeError("a bytes-like object is required, not 'str'")
    if isinstance(data, (bytes, bytearray)):
        raw = bytes(data)
    else:
        try:
            raw = bytes(memoryview(data))
        except TypeError:
            raise TypeError(
                "a bytes-like object is required, not %r" % type(data).__name__
            )
    return _Reader(raw, None, allow_code).read()


def load(file, /, *, allow_code=True):
    probe = file.read(0)
    if not isinstance(probe, bytes):
        raise TypeError("file.read() returned not bytes but " + type(probe).__name__)
    return _Reader(None, file, allow_code).read()


def __getattr__(name):
    raise AttributeError("module 'marshal' has no attribute '" + name + "'")


# Private envelope retained for pickle callers.
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


class _PickleWriter:
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


class _PickleReader:
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
    writer = _PickleWriter(slices, refuse)
    writer.put(value)
    return writer.text().encode()


def _read(data):
    if isinstance(data, str):
        raise TypeError('a str is not written-out bytes')
    if not data:
        raise EOFError('EOF read where object expected')
    return _PickleReader(data.decode()).get()
