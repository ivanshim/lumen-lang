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
                and _stream_write.__code__ is _stream_write_body):
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
    NONE[0]: 0, NEWTRUE[0]: 0, NEWFALSE[0]: 0,
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

def _load_memory(self):
    if (type(self) is not _memory_loader_type or _Unpickler is not _memory_loader_type
            or _Unpickler.load is not _stream_load
            or _stream_load.__code__ is not _stream_load_body
            or _Unframer is not _memory_unframer
            or _Unframer.read is not _memory_unframer_read
            or _Unframer.readline is not _memory_unframer_line
            or _Unframer.__init__ is not _memory_unframer_init
            or io.BytesIO.read is not _memory_stream_read
            or unpack is not _memory_unpack or maxsize != _memory_maxsize
            or sys.gettrace() is not None):
        return self.load()
    if not hasattr(self, "_file_read"):
        return self.load()
    source_read = self._file_read
    if (type(source_read) is not _memory_method_type
            or source_read.__func__ is not _memory_stream_read
            or type(source_read.__self__) is not _memory_stream_type):
        return self.load()
    source_line = self._file_readline
    if (type(source_line) is not _memory_method_type
            or source_line.__func__ is not _memory_stream_line
            or source_line.__self__ is not source_read.__self__):
        return self.load()
    self._unframer = unframer = _Unframer(source_read, source_line)
    self.read = read = unframer.read
    self.readinto = unframer.readinto
    self.readline = readline = unframer.readline
    self.metastack = []
    self.stack = []
    self.append = self.stack.append
    self.proto = 0
    dispatch = self.dispatch
    checked = set()
    try:
        while True:
            append = self.append
            if (type(dispatch) is dict and self.read is read and self.readline is readline
                    and type(self.stack) is list and type(append) is _memory_append_type
                    and append.__self__ is self.stack and append.__name__ == 'append'
                    and io.BytesIO.read is _memory_stream_read
                    and io.BytesIO.readline is _memory_stream_line
                    and unpack is _memory_unpack and maxsize == _memory_maxsize
                    and int is _memory_integer and str is _memory_string
                    and decode_long is _memory_long and decode_long.__code__ is _memory_long_body
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
                                if code not in checked:
                                    handler, body = _scalar_handlers[code]
                                    if dispatch.get(code) is not handler or handler.__code__ is not body:
                                        break
                                    checked.add(code)
                                stream._pos = end
                                if code == NEWTRUE[0]:
                                    argument = True
                                elif code == NEWFALSE[0]:
                                    argument = False
                                append(argument)
                            else:
                                continue
            checked.clear()
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
    if (type(s) is bytes and len(s) >= 256
            and s[0] in (PROTO[0], EMPTY_LIST[0], EMPTY_DICT[0], EMPTY_TUPLE[0])):
        return _load_memory(loader)
    return loader.load()

_loads.__doc__ = _stream_loads.__doc__
if loads is _stream_loads:
    loads = _loads
