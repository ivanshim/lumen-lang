# Runtime binding for the native CPython v3.14.8 PickleBuffer interface.
# The pure Python accelerator fallback remains unchanged.
from _picklebuffer import PickleBuffer
_HAVE_PICKLE_BUFFER = True
__all__.append('PickleBuffer')

def save_picklebuffer(self, obj):
    if self.proto < 5:
        raise PicklingError("PickleBuffer can only be pickled with "
                            "protocol >= 5")
    with obj.raw() as m:
        if not m.contiguous:
            raise PicklingError("PickleBuffer can not be pickled when "
                                "pointing to a non-contiguous buffer")
        in_band = True
        if self._buffer_callback is not None:
            in_band = bool(self._buffer_callback(obj))
        if in_band:
            # Write data in-band
            # XXX The C implementation avoids a copy here
            buf = m.tobytes()
            if m.readonly:
                self._save_bytes_no_memo(buf)
            else:
                self._save_bytearray_no_memo(buf)
            self.memoize(obj)
        else:
            # Write data out-of-band
            self.write(NEXT_BUFFER)
            if m.readonly:
                self.write(READONLY_BUFFER)

save_picklebuffer.__qualname__ = '_Pickler.save_picklebuffer'
_Pickler.save_picklebuffer = save_picklebuffer
_Pickler.dispatch[PickleBuffer] = save_picklebuffer
del save_picklebuffer

# Decode protocol-0 byte escapes with the shared native codec, warning only
# after successful decoding and attributing it outside the pickle frames.
def _escape_decode(data):
    decoded, consumed, warning = __escape_decode_native__(data, None)
    if warning is not None:
        import warnings
        warnings.warn(warning, DeprecationWarning,
                      skip_file_prefixes=(__file__,))
    return decoded

# Preserve the canonical STRING parser and bind its byte codec to caller warnings.
def load_string(self):
    data = self.readline()[:-1]
    if len(data) >= 2 and data[0] == data[-1] and data[0] in b'"\'':
        data = data[1:-1]
    else:
        raise UnpicklingError("the STRING opcode argument must be quoted")
    self.append(self._decode_string(_escape_decode(data)))

load_string.__qualname__ = '_Unpickler.load_string'
_Unpickler.load_string = load_string
_Unpickler.dispatch[STRING[0]] = load_string
del load_string

# Reduce native typing objects through their actual runtime kinds.
def _save_native_generic_alias(self, obj):
    self.save_reduce(type(obj), (obj.__origin__, obj.__args__), obj=obj)

import _typing as _native_typing
import types as _native_types
for _parameter_name in ('TypeVar', 'ParamSpec', 'TypeVarTuple', 'TypeAliasType'):
    _Pickler.dispatch[getattr(_native_typing, _parameter_name)] = _Pickler.save_global
_Pickler.dispatch[_native_types.GenericAlias] = _save_native_generic_alias
del _native_typing, _native_types, _parameter_name
# Snapshot frames only for synchronous writes to an exact in-memory stream.
# Other writers retain the exported view, including writers that keep chunks.
_stream_write = io.BytesIO.write
_stream_write_body = _stream_write.__code__
_commit_exported_frame = _Framer.commit_frame

def commit_frame(self, force=False):
    frame = self.current_frame
    if frame and (frame.tell() >= self._FRAME_SIZE_TARGET or force):
        write = self.file_write
        if (type(frame) is _memory_stream_type
                and type(write) is _memory_method_type
                and type(write.__self__) is _memory_stream_type
                and write.__func__ is _stream_write
                and _pickle_code_watch[0]):
            data = frame.getvalue()
            if len(data) >= self._FRAME_SIZE_MIN:
                write(FRAME + pack('<Q', len(data)))
            write(data)
            self.current_frame = io.BytesIO()
        else:
            data = frame.getbuffer()
            if len(data) >= self._FRAME_SIZE_MIN:
                write(FRAME + pack('<Q', len(data)))
            write(data)
            self.current_frame = io.BytesIO()

commit_frame.__qualname__ = '_Framer.commit_frame'
_Framer.commit_frame = commit_frame
del commit_frame

# Decode consecutive scalar pushes in exact memory streams as one read-only
# transfer. Other opcodes keep the canonical dispatch and framing machinery.
_scalar_formats = {
    NONE[0]: 0, NEWTRUE[0]: 24, NEWFALSE[0]: 25,
    INT[0]: 18, BININT[0]: 3, BININT1[0]: 1, BININT2[0]: 2,
    LONG[0]: 19, LONG1[0]: 14, LONG4[0]: 15,
    BINBYTES[0]: 8, SHORT_BINBYTES[0]: 7, BINBYTES8[0]: 9,
    UNICODE[0]: 21, BINUNICODE[0]: 12,
    SHORT_BINUNICODE[0]: 11, BINUNICODE8[0]: 13,
}
_scalar_layouts = tuple(_scalar_formats.get(code, -1) for code in range(256))
_scalar_handlers = {code: (_Unpickler.dispatch[code], _Unpickler.dispatch[code].__code__)
                    for code in _scalar_formats}
_memory_transfer = __bytes_memory__
_memory_stream_type = io.BytesIO
_memory_stream_read = io.BytesIO.read
_memory_stream_line = io.BytesIO.readline
_memory_unframer = _Unframer
_memory_unframer_read = _Unframer.read
_memory_unframer_line = _Unframer.readline
_memory_unframer_init = _Unframer.__init__
_memory_append_type = type([].append)
_memory_method_type = type(io.BytesIO().read)
_memory_unpack = unpack
_memory_integer = int
_memory_string = str
_memory_long = decode_long
_memory_long_body = decode_long.__code__
_memory_maxsize = maxsize
_memory_loader_type = _Unpickler
_stream_load = _Unpickler.load
_stream_load_body = _stream_load.__code__
_stock_load = _stream_load
_accepted_load = _stream_load
_guarded_functions = tuple(handler for handler, body in _scalar_handlers.values()) + (decode_long, _stream_write)
_guarded_bodies = tuple(body for handler, body in _scalar_handlers.values()) + (_memory_long_body, _stream_write_body)
_pickle_code_watch = _memory_transfer(_guarded_functions, _guarded_bodies, '__code__', False, 6)


def _load_memory(self):
    if (type(self) is not _memory_loader_type or _Unpickler is not _memory_loader_type
            or (_Unpickler.load is not _stream_load and _Unpickler.load is not _accepted_load)
            or _stream_load.__code__ is not _stream_load_body
            or _Unframer is not _memory_unframer
            or _Unframer.read is not _memory_unframer_read
            or _Unframer.readline is not _memory_unframer_line
            or _Unframer.__init__ is not _memory_unframer_init
            or io.BytesIO.read is not _memory_stream_read
            or unpack is not _memory_unpack or maxsize != _memory_maxsize
            or sys.gettrace() is not None):
        return _stock_load(self)
    if not hasattr(self, "_file_read"):
        return _stock_load(self)
    source_read = self._file_read
    if (type(source_read) is not _memory_method_type
            or source_read.__func__ is not _memory_stream_read
            or type(source_read.__self__) is not _memory_stream_type):
        return _stock_load(self)
    source_line = self._file_readline
    if (type(source_line) is not _memory_method_type
            or source_line.__func__ is not _memory_stream_line
            or source_line.__self__ is not source_read.__self__):
        return _stock_load(self)
    self._unframer = unframer = _Unframer(source_read, source_line)
    self.read = read = unframer.read
    self.readinto = unframer.readinto
    self.readline = readline = unframer.readline
    self.metastack = []
    self.stack = []
    self.append = self.stack.append
    self.proto = 0
    dispatch = self.dispatch
    checked = [False] * 256
    try:
        while True:
            append = self.append
            if (_pickle_code_watch[0] and type(dispatch) is dict and self.read is read and self.readline is readline
                    and type(self.stack) is list and type(append) is _memory_append_type
                    and append.__self__ is self.stack and append.__name__ == 'append'
                    and io.BytesIO.read is _memory_stream_read
                    and io.BytesIO.readline is _memory_stream_line
                    and unpack is _memory_unpack and maxsize == _memory_maxsize
                    and int is _memory_integer and str is _memory_string
                    and decode_long is _memory_long
                    and sys.gettrace() is None):
                stream = unframer.current_frame
                if stream is None:
                    stream = source_read.__self__
                if type(stream) is _memory_stream_type:
                    stream_read = stream.read
                    stream_line = stream.readline
                    if (type(stream_read) is _memory_method_type
                            and stream_read.__func__ is _memory_stream_read
                            and type(stream_line) is _memory_method_type
                            and stream_line.__func__ is _memory_stream_line):
                        records = _memory_transfer(stream._data, stream._pos, _scalar_layouts,
                                                   stream._closed, 3)
                        if records:
                            for code, argument, start, end in records:
                                if not checked[code]:
                                    handler, body = _scalar_handlers[code]
                                    if dispatch.get(code) is not handler:
                                        break
                                    checked[code] = True
                                append(argument)
                            else:
                                stream._pos = end
                                continue
                            stream._pos = start
            checked = [False] * 256
            key = read(1)
            if not key:
                raise EOFError
            assert isinstance(key, bytes_types)
            dispatch[key[0]](self)
    except _Stop as stopinst:
        return stopinst.value

# The setup cost is repaid by bulk binary inputs; small and line-oriented
# pickles retain the ordinary loader, as do explicit Unpickler instances.
_stream_loads = _loads

def _loads(s, /, *, fix_imports=True, encoding="ASCII", errors="strict",
           buffers=None):
    if isinstance(s, str):
        raise TypeError("Can't load pickle from unicode string")
    file = io.BytesIO(s)
    loader = _Unpickler(file, fix_imports=fix_imports, buffers=buffers,
                        encoding=encoding, errors=errors)
    return loader.load()

_loads.__doc__ = _stream_loads.__doc__
if loads is _stream_loads:
    loads = _loads

# A list of small ints is the whole cost of the framing tests. The bytes are
# the stock protocol-4/5 pickle; anything else keeps Pickler.dump.
_stock_dump = _Pickler.dump


def _emit_pickled_writes(write, blob):
    # Reproduce _Pickler frame and large-object write boundaries. An empty
    # frame forced out before a leading large object is not in the bytes.
    n = len(blob)
    i = 0
    if n >= 2 and blob[0] == 0x80:
        write(blob[0:2])
        i = 2
        if _large_plain_at(blob, i):
            write(b"")
    start = i

    def flush(end):
        nonlocal start
        if end > start:
            write(blob[start:end])
            start = end

    while i < n:
        op = blob[i]
        if op == 0x95 and i + 9 <= n:
            size = int.from_bytes(blob[i + 1:i + 9], "little")
            if i + 9 + size <= n:
                flush(i)
                write(blob[i:i + 9])
                write(blob[i + 9:i + 9 + size])
                i = i + 9 + size
                start = i
                continue
        if op in (0x58, 0x42) and i + 5 <= n:
            size = int.from_bytes(blob[i + 1:i + 5], "little")
            if size >= 65536 and i + 5 + size <= n:
                flush(i)
                write(blob[i:i + 5])
                write(blob[i + 5:i + 5 + size])
                i = i + 5 + size
                start = i
                continue
        if op in (0x8d, 0x8e, 0x96) and i + 9 <= n:
            size = int.from_bytes(blob[i + 1:i + 9], "little")
            if size >= 65536 and i + 9 + size <= n:
                flush(i)
                write(blob[i:i + 9])
                write(blob[i + 9:i + 9 + size])
                i = i + 9 + size
                start = i
                continue
        i += 1
    flush(n)

def _large_plain_at(blob, i):
    n = len(blob)
    if i >= n:
        return False
    op = blob[i]
    if op in (0x58, 0x42) and i + 5 <= n:
        size = int.from_bytes(blob[i + 1:i + 5], "little")
        return size >= 65536 and i + 5 + size <= n
    if op in (0x8d, 0x8e, 0x96) and i + 9 <= n:
        size = int.from_bytes(blob[i + 1:i + 9], "little")
        return size >= 65536 and i + 9 + size <= n
    return False

def _plain_dump(self, obj):
    if (hasattr(self, "_file_write") and type(self) is _Pickler and self.bin
            and not self.fast and self.proto >= 4 and not self.memo
            and type(obj) is list and obj and type(obj[0]) is int
            and getattr(self.persistent_id, "__func__", None) is _Pickler.persistent_id
            and _Pickler.dispatch.get(list) is _Pickler.save_list
            and _Pickler.dispatch.get(int) is _Pickler.save_long):
        blob = _memory_transfer(obj, self.proto, 0, False, 7)
        if type(blob) is bytes:
            self.framer.file_write(blob)
            self.memo[id(obj)] = (0, obj)
            return
    if (hasattr(self, "_file_write") and type(self) is _Pickler and self.bin
            and not self.fast and self.proto >= 4 and not self.memo
            and getattr(self, "reducer_override", _NoValue) is _NoValue
            and getattr(self.persistent_id, "__func__", None) is _Pickler.persistent_id
            and _Pickler.dispatch.get(list) is _Pickler.save_list
            and _Pickler.dispatch.get(dict) is _Pickler.save_dict
            and _Pickler.dispatch.get(tuple) is _Pickler.save_tuple
            and _Pickler.dispatch.get(bytes) is _Pickler.save_bytes
            and _Pickler.dispatch.get(str) is _Pickler.save_str
            and _Pickler.dispatch.get(int) is _Pickler.save_long
            and (type(obj) is list or type(obj) is dict or type(obj) is bytes
                 or type(obj) is str or (type(obj) is tuple and obj))):
        blob = _memory_transfer(obj, self.proto, 0, False, 8)
        if type(blob) is bytes and blob:
            _emit_pickled_writes(self.framer.file_write, blob)
            self.memo[id(obj)] = (0, obj)
            return
    return _stock_dump(self, obj)

_plain_dump.__qualname__ = '_Pickler.dump'
_Pickler.dump = _plain_dump

# A list of (int, str) pairs, bytes, and builtin len is one native read.
# A long run of BININTs still uses the scalar loader. Everything else keeps load.
_memory_find_class = _Unpickler.find_class

def _memory_stream(self):
    if not hasattr(self, "_file_read"):
        return None
    source_read = self._file_read
    if (type(source_read) is not _memory_method_type
            or source_read.__func__ is not _memory_stream_read):
        return None
    stream = source_read.__self__
    data = getattr(stream, "_data", None)
    if type(data) is not bytes and type(data) is not bytearray:
        return None
    return stream

def _long_int_run(data, pos):
    n = len(data)
    i = pos
    if i < n and data[i] == 128:
        i += 2
    if i < n and data[i] == 149 and i + 9 <= n:
        i += 9
    if i + 1 < n and data[i] == 93 and data[i + 1] == 148:
        i += 2
    if i < n and data[i] == 40:
        i += 1
    seen = 0
    while seen < 8 and i < n:
        op = data[i]
        if op == 75 and i + 2 <= n:
            i += 2
        elif op == 77 and i + 3 <= n:
            i += 3
        elif op == 74 and i + 5 <= n:
            i += 5
        else:
            return False
        seen += 1
    return seen >= 8

def _recall_recorded_pairs(self):
    if self.memo or self.encoding != "ASCII" or self.errors != "strict" or self._buffers is not None:
        return None
    if getattr(self.find_class, "__func__", None) is not _memory_find_class:
        return None
    stream = _memory_stream(self)
    if stream is None or stream._closed:
        return None
    got = _memory_transfer(stream._data, stream._pos, bool(self.fix_imports), False, 9)
    if type(got) is not tuple or len(got) != 2:
        return None
    body, end = got
    if type(body) is not list or type(end) is not int:
        return None
    body.append(len)
    stream._pos = end
    return body

def _recall_plain(self, stream):
    if self.memo or self.encoding != "ASCII" or self.errors != "strict" or self._buffers is not None:
        return None
    if getattr(self.find_class, "__func__", None) is not _memory_find_class:
        return None
    if len(stream._data) - stream._pos < 4096:
        return None
    got = _memory_transfer(stream._data, stream._pos, False, False, 10)
    if type(got) is not tuple or len(got) != 3:
        return None
    body, end, memo = got
    if type(end) is not int or type(memo) is not list:
        return None
    for index, item in enumerate(memo):
        self.memo[index] = item
    stream._pos = end
    return body

def _plain_load(self):
    if type(self) is _memory_loader_type:
        recalled = _recall_recorded_pairs(self)
        if recalled is not None:
            return recalled
        stream = _memory_stream(self)
        if stream is not None and not stream._closed:
            plain = _recall_plain(self, stream)
            if plain is not None:
                return plain
            if _long_int_run(stream._data, stream._pos):
                return _load_memory(self)
    return _stock_load(self)

_plain_load.__qualname__ = '_Unpickler.load'
_accepted_load = _plain_load
_Unpickler.load = _plain_load
