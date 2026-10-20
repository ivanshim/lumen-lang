# Runtime counterpart of CPython v3.14.8 Modules/_io (PSF License).
# Memory storage follows bytesio.c and stringio.c. Buffered text operations
# dispatch to the unchanged _pyio implementation after io finishes importing.
import operator
import sys
from builtins import _buffer_view
_memory = __bytes_memory__
_memory_cursor = __pickle_buffer_native__

DEFAULT_BUFFER_SIZE = 128 * 1024
BlockingIOError = BlockingIOError
_MAXSIZE = sys.maxsize
_MINSIZE = -sys.maxsize - 1

class UnsupportedOperation(OSError, ValueError):
    # Named for the module that exports it, as the reference's own is.
    __module__ = 'io'

def text_encoding(encoding, stacklevel=2):
    if encoding is None:
        encoding = 'utf-8' if sys.flags.utf8_mode else 'locale'
        if sys.flags.warn_default_encoding:
            import warnings
            warnings.warn("'encoding' argument not specified.", EncodingWarning, stacklevel + 1)
    return encoding

def _index(value):
    if type(value) is int:
        return value
    if type(value) is bool:
        return int(value)
    if not hasattr(type(value), '__index__'):
        raise TypeError("'" + type(value).__name__ + "' object cannot be interpreted as an integer")
    return operator.index(value)

def _size(value):
    if type(value) is int and _MINSIZE <= value <= _MAXSIZE:
        return value
    if value is None:
        return -1
    if type(value) is int:
        result = value
    else:
        if not hasattr(type(value), '__index__'):
            raise TypeError("argument should be integer or None, not '" + type(value).__name__ + "'")
        result = _index(value)
    if result > _MAXSIZE or result < _MINSIZE:
        raise OverflowError("cannot fit '" + type(value).__name__ + "' into an index-sized integer")
    return result

def _ssize(value):
    if type(value) is int and _MINSIZE <= value <= _MAXSIZE:
        return value
    value = _index(value)
    if value > _MAXSIZE or value < _MINSIZE:
        raise OverflowError('Python int too large to convert to C ssize_t')
    return value

def _whence(value):
    if type(value) is int and -2147483648 <= value <= 2147483647:
        return value
    value = _index(value)
    if value < -2147483648 or value > 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    return value

def _get_buffer(value):
    from array import array
    if not isinstance(value, (bytes, bytearray, memoryview, array)) and not hasattr(type(value), '__buffer__'):
        raise TypeError("a bytes-like object is required, not '" + type(value).__name__ + "'")
    return _buffer_view(value, 8)

def _buffer(value):
    kind = type(value)
    if kind is bytes:
        return value
    if kind is bytearray:
        return bytes(value)
    with _get_buffer(value) as view:
        if not view.c_contiguous:
            raise BufferError('memoryview: underlying buffer is not C-contiguous')
        return view.tobytes()

class _IOBase:
    _closed = False
    # Set while the object is being finalised, so that close can say an
    # open stream was dropped without being closed.
    _finalizing = False
    def __del__(self):
        try:
            closed = self.closed
        except (AttributeError, ValueError):
            return
        if not closed:
            try:
                dealloc_warn = getattr(self, '_dealloc_warn', None)
                if dealloc_warn is not None:
                    dealloc_warn(self)
            finally:
                self.close()
    def _dealloc_warn(self, source):
        return None
    def _checkClosed(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')
    def _checkReadable(self):
        if not self.readable():
            raise UnsupportedOperation('File or stream is not readable.')
    def _checkWritable(self):
        if not self.writable():
            raise UnsupportedOperation('File or stream is not writable.')
    def _checkSeekable(self):
        if not self.seekable():
            raise UnsupportedOperation('File or stream is not seekable.')
    @property
    def closed(self):
        """True if the file is closed"""
        return self._closed
    def flush(self):
        self._checkClosed()
    def close(self):
        if not self.closed:
            try:
                self.flush()
            finally:
                self._closed = True
    def readable(self):
        return False
    def writable(self):
        return False
    def seekable(self):
        return False
    def fileno(self):
        raise UnsupportedOperation('fileno')
    def isatty(self):
        self._checkClosed()
        return False
    def seek(self, offset, whence=0):
        raise UnsupportedOperation('seek')
    def tell(self):
        return self.seek(0, 1)
    def truncate(self, size=None):
        raise UnsupportedOperation('truncate')
    def __enter__(self):
        self._checkClosed()
        return self
    def __exit__(self, *args):
        self.close()
    def __iter__(self):
        self._checkClosed()
        return self
    def __next__(self):
        line = self.readline()
        if not line:
            raise StopIteration
        return line
    def readline(self, size=-1):
        size = _size(size)
        result = b''
        while size < 0 or len(result) < size:
            part = self.read(1)
            if not part:
                break
            result += part
            if part == b'\n':
                break
        return result
    def readlines(self, hint=-1):
        hint = _size(hint)
        lines = []
        total = 0
        for line in self:
            lines.append(line)
            total += len(line)
            if hint > 0 and total > hint:
                break
        return lines
    def writelines(self, lines):
        self._checkClosed()
        for line in lines:
            self.write(line)
        return None

class _RawIOBase(_IOBase):
    def read(self, size=-1):
        size = _size(size)
        if size < 0:
            return self.readall()
        buf = bytearray(size)
        n = self.readinto(buf)
        if n is None:
            return None
        n = _index(n)
        if n < 0 or n > size:
            raise ValueError('readinto returned %d outside buffer size %d' % (n, size))
        return bytes(buf[:n])
    def readall(self):
        result = b''
        while True:
            part = self.read(DEFAULT_BUFFER_SIZE)
            if not part:
                return result if result else part
            result += part
    def readinto(self, b):
        raise NotImplementedError
    def write(self, b):
        raise NotImplementedError

class _BufferedIOBase(_IOBase):
    def read(self, size=-1):
        raise UnsupportedOperation('read')
    def read1(self, size=-1):
        raise UnsupportedOperation('read1')
    def write(self, b):
        raise UnsupportedOperation('write')
    def detach(self):
        raise UnsupportedOperation('detach')
    def _readinto_with(self, b, name):
        with memoryview(b) as view:
            target = view.cast('B')
            if target.readonly:
                raise TypeError('readinto() argument must be read-write bytes-like object')
            data = getattr(self, name)(target.nbytes)
            if not isinstance(data, bytes):
                raise TypeError('read() should return bytes')
            if len(data) > target.nbytes:
                raise ValueError('read() returned too much data: %d bytes requested, %d returned' % (target.nbytes, len(data)))
            target[:len(data)] = data
            return len(data)
    def readinto(self, b):
        return self._readinto_with(b, 'read')
    def readinto1(self, b):
        return self._readinto_with(b, 'read1')

class _TextIOBase(_IOBase):
    @property
    def encoding(self):
        return None
    @property
    def errors(self):
        return None
    @property
    def newlines(self):
        return None
    def read(self, size=-1):
        raise UnsupportedOperation('read')
    def readline(self, size=-1):
        raise UnsupportedOperation('readline')
    def write(self, text):
        raise UnsupportedOperation('write')
    def detach(self):
        raise UnsupportedOperation('detach')

class _BytesIOBuffer:
    def __init__(self, owner):
        self._owner = owner
    def __buffer__(self, flags):
        return memoryview(self._owner._data)

class BytesIO(_BufferedIOBase):
    __slots__ = ('_data', '_pos', '_closed')
    def __new__(cls, *args, **kwargs):
        obj = object.__new__(cls)
        obj._data = bytearray()
        obj._pos = 0
        obj._closed = False
        return obj
    def _checkClosed(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')
    def __init__(self, initial_bytes=None):
        if hasattr(self, '_data'):
            if _export(self._data, True):
                raise BufferError('Existing exports of data: object cannot be re-sized')
        self._data = bytearray() if initial_bytes is None else bytearray(_buffer(initial_bytes))
        self._pos = 0
        self._closed = False
    def _check_exports(self):
        if _export(self._data, True):
            raise BufferError('Existing exports of data: object cannot be re-sized')
    def getvalue(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')
        return bytes(self._data)
    def getbuffer(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')
        view = memoryview(self._data)
        view._object = _BytesIOBuffer(self)
        return view
    def close(self):
        if not self.closed:
            if _export(self._data, True):
                raise BufferError('Existing exports of data: object cannot be re-sized')
            self._data.clear()
            self._closed = True
    def readable(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')
        return True
    writable = readable
    seekable = readable
    def read(self, size=-1):
        data = _memory_cursor(6, self, size)
        if data is None:
            data = _memory_cursor(6, self, _size(size))
        return data
    read1 = read
    def readinto(self, buffer):
        if type(buffer) is bytearray:
            data = BytesIO.read(self, len(buffer))
            buffer[:len(data)] = data
            return len(data)
        with memoryview(buffer) as view:
            target = view.cast('B')
            if target.readonly:
                raise TypeError('readinto() argument must be read-write bytes-like object')
            data = BytesIO.read(self, target.nbytes)
            target[:len(data)] = data
            return len(data)
    readinto1 = readinto
    def readline(self, size=-1):
        if type(size) is int:
            result, self._pos = _memory(self._data, self._pos, size, self._closed, 1)
            return result
        size = _size(size)
        if self._closed:
            raise ValueError('I/O operation on closed file')
        end = len(self._data) if size < 0 else min(len(self._data), self._pos + size)
        start = self._pos
        if end <= start:
            return b''
        at = self._data.find(b'\n', start, end)
        if at >= 0:
            end = at + 1
        data = bytes(self._data[start:end])
        self._pos = end
        return data
    def write(self, b):
        stored = _memory_cursor(7, self, b)
        if stored is not None:
            return stored
        with _get_buffer(b) as view:
            if not view.c_contiguous:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            if self._closed:
                raise ValueError('I/O operation on closed file')
            if _export(self._data, True):
                raise BufferError('Existing exports of data: object cannot be re-sized')
            return self._store(view.tobytes())
    def _store(self, data):
        return _memory_cursor(7, self, data)
    def seek(self, pos, whence=0):
        if type(pos) is not int or pos > _MAXSIZE or pos < _MINSIZE:
            pos = _ssize(pos)
        if type(whence) is not int or whence > 2147483647 or whence < -2147483648:
            whence = _whence(whence)
        if self._closed:
            raise ValueError('I/O operation on closed file')
        if whence == 0:
            if pos < 0:
                raise ValueError('negative seek value ' + str(pos))
        elif whence == 1:
            pos += self._pos
        elif whence == 2:
            pos += len(self._data)
        else:
            raise ValueError('invalid whence (' + str(whence) + ', should be 0, 1 or 2)')
        if pos > _MAXSIZE:
            raise OverflowError('new position too large')
        self._pos = max(0, pos)
        return self._pos
    def tell(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')
        return self._pos
    def truncate(self, size=None):
        size = self._pos if size is None else _ssize(size)
        if self._closed:
            raise ValueError('I/O operation on closed file')
        if _export(self._data, True):
            raise BufferError('Existing exports of data: object cannot be re-sized')
        if size < 0:
            raise ValueError('negative size value ' + str(size))
        del self._data[size:]
        return size
    def __getstate__(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')
        return self.getvalue(), self._pos, self.__dict__.copy()
    def __setstate__(self, state):
        if getattr(self, '_closed', False):
            raise ValueError('I/O operation on closed file')
        if not isinstance(state, tuple) or len(state) < 3:
            raise TypeError('state should be a 3-tuple')
        data, pos, attrs = state[:3]
        pos = _ssize(pos)
        if pos < 0:
            raise ValueError('position value cannot be negative')
        if attrs is not None and not isinstance(attrs, dict):
            raise TypeError('third item of state should be a dict')
        BytesIO.__init__(self, data)
        self._pos = pos
        if attrs is not None:
            self.__dict__.update(attrs)

class StringIO(_TextIOBase):
    __slots__ = ('_text', '_chunks', '_length', '_pos', '_closed', '_newline', '_seen')
    @property
    def closed(self):
        if not hasattr(self, '_text'):
            raise ValueError('I/O operation on uninitialized object')
        return self._closed
    def _checkClosed(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')
    @property
    def line_buffering(self):
        self._checkClosed()
        return False
    def __init__(self, initial_value='', newline='\n'):
        if initial_value is not None and not isinstance(initial_value, str):
            raise TypeError('initial_value must be str or None, not ' + type(initial_value).__name__)
        if newline is not None and not isinstance(newline, str):
            raise TypeError('newline must be str or None, not ' + type(newline).__name__)
        if newline not in (None, '', '\n', '\r', '\r\n'):
            raise ValueError('illegal newline value: ' + repr(newline))
        self._newline = newline
        self._seen = 0
        self._text = ''
        self._chunks = []
        self._length = 0
        self._pos = 0
        self._closed = False
        if initial_value:
            self.write(initial_value)
            self._pos = 0
    def _materialize(self):
        if self._chunks:
            self._text += ''.join(self._chunks)
            self._chunks = []
    def _translate(self, text):
        if self._newline in (None, ''):
            crlf = text.count('\r\n')
            if crlf:
                self._seen |= 4
            if text.count('\r') > crlf:
                self._seen |= 2
            if text.count('\n') > crlf:
                self._seen |= 1
            if self._newline is None:
                return text.replace('\r\n', '\n').replace('\r', '\n')
        elif self._newline != '\n':
            return text.replace('\n', self._newline)
        return text
    @property
    def newlines(self):
        self._checkClosed()
        return (None, '\n', '\r', ('\r', '\n'), '\r\n', ('\n', '\r\n'), ('\r', '\r\n'), ('\r', '\n', '\r\n'))[self._seen]
    def getvalue(self):
        self._checkClosed()
        self._materialize()
        return self._text
    def readable(self):
        self._checkClosed()
        return True
    writable = readable
    seekable = readable
    def close(self):
        self._chunks = []
        self._length = 0
        self._text = ''
        self._closed = True
    def write(self, text):
        if not isinstance(text, str):
            raise TypeError('string argument expected, got ' + type(text).__name__)
        self._checkClosed()
        n = len(text)
        if not n:
            return 0
        data = self._translate(text)
        if self._pos == self._length:
            self._chunks.append(data)
            self._length += len(data)
            self._pos = self._length
            return n
        self._materialize()
        if self._pos > len(self._text):
            self._text += '\0' * (self._pos - len(self._text))
        self._text = self._text[:self._pos] + data + self._text[self._pos + len(data):]
        self._pos += len(data)
        self._length = len(self._text)
        return n
    def read(self, size=-1):
        size = _size(size)
        self._checkClosed()
        self._materialize()
        end = len(self._text) if size < 0 else min(len(self._text), self._pos + size)
        if end <= self._pos:
            return ''
        data = self._text[self._pos:end]
        self._pos = end
        return data
    def readline(self, size=-1):
        size = _size(size)
        self._checkClosed()
        self._materialize()
        if self._newline != '':
            start = self._pos
            end = len(self._text)
            if start >= end:
                return ''
            newline = self._newline or '\n'
            found = self._text.find(newline, start)
            if found >= 0:
                end = found + len(newline)
            if size >= 0:
                end = min(end, start + size)
            self._pos = end
            return self._text[start:end]
        start = self._pos
        length = len(self._text)
        if start >= length:
            return ''
        cr = self._text.find('\r', start)
        lf = self._text.find('\n', start)
        stop = length
        if cr >= 0 and (lf < 0 or cr < lf):
            stop = cr + 1
            if self._text[stop:stop + 1] == '\n':
                stop += 1
        elif lf >= 0:
            stop = lf + 1
        if size >= 0:
            stop = min(stop, start + size)
        self._pos = stop
        return self._text[start:stop]
    def seek(self, pos, whence=0):
        pos = _ssize(pos)
        whence = _whence(whence)
        self._checkClosed()
        if whence not in (0, 1, 2):
            raise ValueError('Invalid whence (' + str(whence) + ', should be 0, 1 or 2)')
        if whence == 0:
            if pos < 0:
                raise ValueError('Negative seek position ' + str(pos))
        else:
            if pos:
                raise OSError("Can't do nonzero cur-relative seeks")
            pos = self._pos if whence == 1 else self._length
        self._pos = pos
        return pos
    def tell(self):
        self._checkClosed()
        return self._pos
    def truncate(self, size=None):
        size = self._pos if size is None else _ssize(size)
        self._checkClosed()
        if size < 0:
            raise ValueError('Negative size value ' + str(size))
        self._materialize()
        self._text = self._text[:size]
        self._length = len(self._text)
        return size
    def __getstate__(self):
        self._checkClosed()
        self._materialize()
        return self._text, self._newline, self._pos, self.__dict__.copy()
    def __setstate__(self, state):
        if getattr(self, '_closed', False):
            raise ValueError('I/O operation on closed file')
        if not isinstance(state, tuple) or len(state) < 4:
            raise TypeError('state should be a 4-tuple')
        text, newline, pos, attrs = state[:4]
        if not isinstance(text, str):
            raise TypeError('first item of state should be a str')
        pos = _ssize(pos)
        if pos < 0:
            raise ValueError('position value cannot be negative')
        if attrs is not None and not isinstance(attrs, dict):
            raise TypeError('fourth item of state should be a dict')
        StringIO.__init__(self, '', newline)
        self._text = text
        self._length = len(text)
        self._pos = pos
        if attrs is not None:
            self.__dict__.update(attrs)

# These bridge methods call the original Python algorithms with this object's
# state. Import is delayed because io declares the ABCs that _pyio registers.
_methods = {}

_pyio_module = None

def _text_runtime():
    global _pyio_module
    if _pyio_module is None:
        import _pyio as module
        _pyio_module = module
    return _pyio_module

def _method(instance, kind, name):
    member = _methods.get((kind, name))
    if member is None:
        _pyio = _text_runtime()
        member = _methods[(kind, name)] = getattr(getattr(_pyio, kind), name)
    return member.__get__(instance, type(instance))

def _text_option(value, name):
    if value is not None:
        if not isinstance(value, str):
            raise TypeError(name + ' must be str or None, not ' + type(value).__name__)
        value.encode('utf-8')
    return value

_TEXT = {}
_TEXT_HELPERS = []

def _install_text_helpers():
    # The reference's text methods call a dozen helpers on the stream
    # itself (the decoded-chars pair, the chunk reader, the cookie
    # packers). Left to __getattr__ each call cost three; standing in the
    # class they are found as any method is.
    _pyio = _text_runtime()
    _TEXT_HELPERS.append(True)
    own = vars(TextIOWrapper)
    for name, value in list(vars(_pyio.TextIOWrapper).items()):
        if name in own or (name.startswith('__') and name.endswith('__')) or name.startswith('_abc'):
            continue
        if type(value) is type(_install_text_helpers) or type(value) is property:
            setattr(TextIOWrapper, name, value)

def _text_function(name):
    _pyio = _text_runtime()
    function = _TEXT[name] = getattr(_pyio.TextIOWrapper, name)
    return function

class TextIOWrapper(_TextIOBase):
    # Read without a trip through __getattr__ before the stream is made.
    _initialized = False
    _buffer = None
    # Whether the buffer could be read and written when the stream was
    # made, which is when the C stream settles it.
    _read_ok = False
    _write_ok = False
    # Encoded text waiting to go to the buffer, and its size: the C stream
    # collects writes until a chunk is full, or something asks for them.
    _pending = None
    _pending_count = 0
    _chunk_size = 8192
    def _dealloc_warn(self, source):
        try:
            self.buffer._dealloc_warn(source)
        except Exception:
            pass
    @property
    def _CHUNK_SIZE(self):
        return self._chunk_size
    @_CHUNK_SIZE.setter
    def _CHUNK_SIZE(self, value):
        value = _index(value)
        if value <= 0 or value > sys.maxsize:
            raise ValueError('a strictly positive integer is required')
        self._chunk_size = value
    @_CHUNK_SIZE.deleter
    def _CHUNK_SIZE(self):
        raise AttributeError('cannot be deleted')
    def __init__(self, buffer, encoding=None, errors=None, newline=None, line_buffering=False, write_through=False):
        # Queued bytes belong to the current backing stream. Drain them
        # before initialization can replace that stream or its codec state.
        if self._pending:
            self.flush()
        self._pending = []
        self._pending_count = 0
        _pyio = _text_runtime()
        if not _TEXT_HELPERS:
            _install_text_helpers()
        self._initialized = False
        encoding = _text_option(encoding, 'encoding')
        errors = _text_option(errors, 'errors')
        if encoding is None:
            encoding = text_encoding(None)
        if (encoding is not None and '\0' in encoding) or (errors is not None and '\0' in errors):
            raise ValueError('embedded null character')
        line_buffering = bool(_whence(line_buffering))
        write_through = bool(_whence(write_through))
        self._initialized = True
        try:
            _pyio.TextIOWrapper.__init__(self, buffer, encoding, errors, newline, line_buffering, write_through)
        except BaseException:
            self._initialized = False
            raise
        try:
            self._read_ok = bool(buffer.readable())
            self._write_ok = bool(buffer.writable())
        except Exception:
            self._read_ok = self._write_ok = False
    def __getstate__(self):
        raise TypeError("cannot pickle '" + type(self).__name__ + "' instances")
    def __repr__(self):
        if not getattr(self, '_initialized', False):
            raise ValueError('I/O operation on uninitialized object')
        title = type(self).__module__ + '.' + type(self).__qualname__
        if getattr(self, '_repr_running', False):
            raise RuntimeError('reentrant call inside ' + title + '.__repr__')
        self._repr_running = True
        try:
            result = '<' + title
            try:
                name = self.name
            except (AttributeError, ValueError):
                pass
            else:
                result += ' name={!r}'.format(name)
            try:
                mode = self.mode
            except AttributeError:
                pass
            else:
                result += ' mode={!r}'.format(mode)
            return result + ' encoding={!r}>'.format(self.encoding)
        finally:
            self._repr_running = False
    def reconfigure(self, *, encoding=None, errors=None, newline=Ellipsis, line_buffering=None, write_through=None):
        encoding = _text_option(encoding, 'encoding')
        errors = _text_option(errors, 'errors')
        if encoding is not None and encoding != 'locale':
            codecs = _text_runtime().codecs
            codecs.lookup(encoding.split('\0', 1)[0])
        if line_buffering is not None:
            line_buffering = bool(_whence(line_buffering))
        if write_through is not None:
            write_through = bool(_whence(write_through))
        return _method(self, 'TextIOWrapper', 'reconfigure')(encoding=encoding, errors=errors, newline=newline, line_buffering=line_buffering, write_through=write_through)
    def _get_encoder(self):
        codecs = _text_runtime().codecs
        factory = codecs.getincrementalencoder(self._encoding.split('\0', 1)[0])
        self._encoder = factory(self._errors.split('\0', 1)[0])
        return self._encoder
    def _get_decoder(self):
        codecs = _text_runtime().codecs
        create = codecs.getincrementaldecoder(self._encoding.split('\0', 1)[0])
        decoded = create(self._errors.split('\0', 1)[0])
        if self._readuniversal:
            decoded = IncrementalNewlineDecoder(decoded, self._readtranslate)
        self._decoder = decoded
        return decoded
    def __getattr__(self, name):
        if name in ('_initialized', '_repr_running', '_chunk_size'):
            raise AttributeError(name)
        return _method(self, 'TextIOWrapper', name)
    def _unattached(self):
        if not self._initialized:
            raise ValueError('I/O operation on uninitialized object')
        raise ValueError('underlying buffer has been detached')
    @property
    def buffer(self):
        # After a detach the attribute reads None, as the C stream's does;
        # the methods say the buffer is gone.
        if not self._initialized:
            raise ValueError('I/O operation on uninitialized object')
        return self._buffer
    @property
    def name(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        return buffer.name
    @property
    def line_buffering(self):
        return self._line_buffering
    @property
    def write_through(self):
        return self._write_through
    @property
    def encoding(self):
        return self._encoding
    @property
    def errors(self):
        return self._errors
    @property
    def newlines(self):
        if self._buffer is None or not self._initialized:
            self._unattached()
        return self._decoder.newlines if self._decoder and self._readuniversal else None
    @property
    def closed(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        kind = type(buffer)
        if kind is BufferedWriter or kind is BufferedReader or kind is BufferedRandom:
            impl = buffer._live
            if impl is not None:
                checked = impl.raw
                if not checked._owned:
                    return True
                return checked.raw._closed if checked._plain else checked.raw.closed
        return buffer.closed
    def _writeflush(self):
        pending = self._pending
        if not pending:
            return
        # The count and the list are cleared before the buffer is written
        # to, so a write made from inside that call starts a fresh one.
        self._pending = []
        self._pending_count = 0
        self.buffer.write(pending[0] if len(pending) == 1 else b''.join(pending))
    def read(self, size=-1):
        if not self._read_ok and not self.readable():
            raise UnsupportedOperation('not readable')
        if self._pending:
            self._writeflush()
        function = _TEXT.get('read') or _text_function('read')
        return function(self, size)
    def readline(self, size=-1):
        if self._buffer is None or not self._initialized:
            self._unattached()
        if self._pending:
            self._writeflush()
        function = _TEXT.get('readline') or _text_function('readline')
        return function(self, size)
    def write(self, text):
        if not self._write_ok and not self.writable():
            raise UnsupportedOperation('not writable')
        if self.closed:
            raise ValueError('write to closed file')
        if not isinstance(text, str):
            raise TypeError("can't write %s to text stream" % text.__class__.__name__)
        length = len(text)
        haslf = (self._writetranslate or self._line_buffering) and '\n' in text
        if haslf and self._writetranslate and self._writenl != '\n':
            text = text.replace('\n', self._writenl)
        encoder = self._encoder or self._get_encoder()
        data = encoder.encode(text)
        if not isinstance(data, bytes):
            raise TypeError("encoder should return a bytes object, not '%s'" % type(data).__name__)
        chunk = self._chunk_size
        # A write that would take the waiting data past a chunk sends what
        # waits first, and goes again in case that call wrote something.
        while self._pending and self._pending_count + len(data) > chunk:
            self._writeflush()
        if self._pending is None:
            self._pending = []
        self._pending.append(data)
        self._pending_count += len(data)
        if self._pending_count >= chunk or self._write_through:
            self._writeflush()
        if self._line_buffering and (haslf or '\r' in text):
            self.flush()
        if self._snapshot is not None:
            self._set_decoded_chars('')
            self._snapshot = None
        if self._decoder:
            self._decoder.reset()
        return length
    def seek(self, cookie, whence=0):
        if self._buffer is None or not self._initialized:
            self._unattached()
        function = _TEXT.get('seek') or _text_function('seek')
        return function(self, cookie, whence)
    def tell(self):
        if self._buffer is None or not self._initialized:
            self._unattached()
        function = _TEXT.get('tell') or _text_function('tell')
        return function(self)
    def truncate(self, pos=None):
        if self._buffer is None or not self._initialized:
            self._unattached()
        function = _TEXT.get('truncate') or _text_function('truncate')
        return function(self, pos)
    def readable(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        return buffer.readable()
    def writable(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        return buffer.writable()
    def seekable(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        return buffer.seekable()
    def flush(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        if self._pending:
            self._writeflush()
        # The reference flushes the buffer and then marks telling possible.
        buffer.flush()
        self._telling = self._seekable
    # Close the backing stream even when finalization warning lookup fails.
    def close(self):
        if self._buffer is None or not self._initialized:
            self._unattached()
        attached = self._buffer
        try:
            if self._finalizing and not attached.closed:
                warn = getattr(attached, '_dealloc_warn', None)
                if warn is not None:
                    warn(self)
        finally:
            try:
                # The reference's own close: flush, then close the buffer,
                # unless there is none or it is closed already.
                result = None
                if self.buffer is not None and not self.closed:
                    try:
                        self.flush()
                    finally:
                        self.buffer.close()
            except Exception:
                # A buffer detached inside its closed check needs no closure.
                if attached is not None and self._buffer is None and self._initialized:
                    result = None
                else:
                    raise
        return result
    def detach(self):
        if self._buffer is None or not self._initialized:
            self._unattached()
        function = _TEXT.get('detach') or _text_function('detach')
        return function(self)
    def fileno(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        return buffer.fileno()
    def isatty(self):
        buffer = self._buffer
        if buffer is None or not self._initialized:
            self._unattached()
        return buffer.isatty()
    def __next__(self):
        if self._buffer is None or not self._initialized:
            self._unattached()
        if self._pending:
            self._writeflush()
        function = _TEXT.get('__next__') or _text_function('__next__')
        return function(self)

class IncrementalNewlineDecoder:
    def __init__(self, decoder, translate, errors='strict'):
        _pyio = _text_runtime()
        self._impl = _pyio.IncrementalNewlineDecoder(decoder, translate, errors)
    def decode(self, input, final=False):
        return self._impl.decode(input, final)
    def getstate(self):
        return self._impl.getstate()
    def setstate(self, state):
        return self._impl.setstate(state)
    def reset(self):
        return self._impl.reset()
    @property
    def newlines(self):
        return self._impl.newlines

_PLAIN_MODES = frozenset(['r', 'rb', 'w', 'wb', 'a', 'ab', 'x', 'xb', 'r+', 'rb+', 'r+b', 'w+', 'wb+', 'w+b', 'a+', 'ab+', 'a+b', 'x+', 'xb+', 'x+b'])

_finalizer_warnings = None
_finalizer_collector = None
_finalizer_os = None

def _file_plans():
    # What each plain mode string works out to: readable, writable, the
    # mode the stream reports, the open flags and whether it appends. The
    # general road below derives the same from any spelling.
    import os
    plans = {}
    for letters in ('r', 'w', 'a', 'x', 'r+', 'w+', 'a+', 'x+', '+r', '+w', '+a', '+x', 'rb', 'wb', 'ab', 'xb', 'r+b', 'w+b', 'a+b', 'x+b', 'rb+', 'wb+', 'ab+', 'xb+', 'br', 'bw', 'ba', 'bx', 'b+r', 'b+w', 'b+a', 'b+x', 'br+', 'bw+', 'ba+', 'bx+', '+rb', '+wb', '+ab', '+xb', '+br', '+bw', '+ba', '+bx'):
        letter = [one for one in letters if one in 'rwax'][0]
        plus = '+' in letters
        flags = os.O_RDWR if plus else os.O_RDONLY if letter == 'r' else os.O_WRONLY
        if letter != 'r':
            flags |= os.O_CREAT
        if letter == 'w':
            flags |= os.O_TRUNC
        if letter == 'x':
            flags |= os.O_EXCL
        if letter == 'a':
            flags |= os.O_APPEND
        reported = ('ab+' if letter == 'a' else 'xb+' if letter == 'x' else 'rb+') if plus else ('ab' if letter == 'a' else 'xb' if letter == 'x' else 'rb' if letter == 'r' else 'wb')
        plans[letters] = (letter == 'r' or plus, letter != 'r' or plus, reported, flags | os.O_CLOEXEC, letter == 'a')
    return plans

_FILE_PLANS = {}

class FileIO(_RawIOBase):
    # What a stream whose __init__ never finished stands as, the way the C
    # stream stands with no descriptor: closed, and owing nothing.
    _fd = -1
    _closed = True
    _closefd = True
    _readable = False
    _writable = False
    _created = False
    _appending = False
    _finalizing_state = False
    def __init__(self, file, mode='r', closefd=True, opener=None):
        if type(file) is str and type(mode) is str and closefd is True and opener is None:
            if not _FILE_PLANS:
                _FILE_PLANS.update(_file_plans())
            plan = _FILE_PLANS.get(mode)
            if plan is not None:
                self._init_named(file, plan)
                return
        import os
        if getattr(self, '_fd', -1) >= 0:
            self.close()
        self._fd = -1
        self._closed = True
        self._finalizing_state = False
        if not isinstance(mode, str):
            raise TypeError('mode must be a string')
        if any(letter not in 'rwax+b' for letter in mode):
            raise ValueError('invalid mode: ' + mode)
        base = [letter for letter in mode if letter in 'rwax']
        if len(base) != 1 or mode.count('+') > 1:
            raise ValueError('Must have exactly one of create/read/write/append mode and at most one plus')
        self._readable = base[0] == 'r' or '+' in mode
        self._writable = base[0] != 'r' or '+' in mode
        self._closefd = bool(closefd)
        self._fd = -1
        self._closed = True
        if isinstance(file, float):
            raise TypeError('integer argument expected, got float')
        if isinstance(file, bool):
            import warnings
            warnings.warn('bool is used as a file descriptor', RuntimeWarning, stacklevel=2)
            file = int(file)
        self._created = base[0] == 'x'
        self._appending = base[0] == 'a'
        owned = False
        fd = -1
        try:
            if isinstance(file, int) or hasattr(type(file), '__index__'):
                fd = _index(file)
                if fd < 0:
                    raise ValueError('negative file descriptor')
                if fd > 2147483647:
                    raise OverflowError('Python int too large to convert to C int')
                os.get_inheritable(fd)
            else:
                if not closefd:
                    raise ValueError('Cannot use closefd=False with file name')
                flags = os.O_RDWR if '+' in mode else os.O_RDONLY if base[0] == 'r' else os.O_WRONLY
                if base[0] in ('w', 'x', 'a'):
                    flags |= os.O_CREAT
                if base[0] == 'w': flags |= os.O_TRUNC
                if base[0] == 'x': flags |= os.O_EXCL
                if base[0] == 'a': flags |= os.O_APPEND
                flags |= os.O_CLOEXEC
                if opener is None:
                    fd = os.open(file, flags, 438)
                else:
                    fd = opener(file, flags)
                    if not isinstance(fd, int):
                        raise TypeError('expected integer from opener')
                    fd = _index(fd)
                    if fd < -2147483648 or fd > 2147483647:
                        raise OverflowError('Python int too large to convert to C int')
                    if fd < 0:
                        raise ValueError('opener returned ' + str(fd))
                owned = True
                os.set_inheritable(fd, False)
            # The C code asks the system directly, so a replaced os.fstat is not seen here.
            import posix
            try:
                file_mode = posix.fstat(fd).st_mode
            except OSError as exc:
                if exc.errno == 9:
                    raise
            else:
                if file_mode & 61440 == 16384:
                    raise IsADirectoryError(21, 'Is a directory', file)
            self.name = file
            self._fd = fd
            self._closed = False
            global _finalizer_warnings, _finalizer_collector, _finalizer_os
            if _finalizer_warnings is None:
                import warnings, gc
                _finalizer_warnings, _finalizer_collector, _finalizer_os = warnings, gc, os
            if base[0] == 'a':
                try:
                    self.seek(0, 2)
                except OSError as exc:
                    if exc.errno != 29:
                        raise
        except BaseException:
            self._fd = -1
            self._closed = True
            if owned and fd >= 0:
                try:
                    os.close(fd)
                except OSError:
                    pass
            raise
    def _init_named(self, file, plan):
        # The common case of __init__: a path given as text, a plain mode.
        # Match the general initializer: raw descriptor checks bypass replaceable os wrappers.
        import posix
        global _finalizer_warnings, _finalizer_collector, _finalizer_os
        os = _finalizer_os
        if os is None:
            import os, warnings, gc
            if _finalizer_warnings is None:
                _finalizer_warnings, _finalizer_collector, _finalizer_os = warnings, gc, os
        if self._fd >= 0:
            self.close()
        self._fd = -1
        self._closed = True
        self._finalizing_state = False
        self._readable, self._writable, reported, flags, appending = plan
        self._created = reported[0] == 'x'
        self._appending = appending
        self._closefd = True
        fd = -1
        try:
            fd = os.open(file, flags, 438)
            try:
                file_mode = posix.fstat(fd).st_mode
            except OSError as exc:
                if exc.errno == 9:
                    raise
            else:
                if file_mode & 61440 == 16384:
                    raise IsADirectoryError(21, 'Is a directory', file)
            self.name = file
            self._fd = fd
            self._closed = False
            if appending:
                try:
                    self.seek(0, 2)
                except OSError as exc:
                    if exc.errno != 29:
                        raise
        except BaseException:
            self._fd = -1
            self._closed = True
            if fd >= 0:
                try:
                    os.close(fd)
                except OSError:
                    pass
            raise
    def _dealloc_warn(self, source):
        if getattr(self, '_fd', -1) >= 0 and getattr(self, '_closefd', False):
            if sys.is_finalizing() and _finalizer_warnings is not None:
                warnings, gc = _finalizer_warnings, _finalizer_collector
            else:
                import warnings
                import gc
            prefixes = (__file__,)
            collector_source = getattr(gc, '__file__', None)
            if collector_source is not None:
                prefixes += (collector_source,)
            try:
                warnings.warn('unclosed file %r' % (source,), ResourceWarning,
                              stacklevel=2, source=source, skip_file_prefixes=prefixes)
            except Warning as error:
                sys._report_unraisable(error, error.__traceback__, self, 'file')
            except Exception:
                pass
    def __getstate__(self):
        raise TypeError("cannot pickle 'FileIO' instances")
    def __repr__(self):
        title = type(self).__module__ + '.' + type(self).__qualname__
        if getattr(self, '_fd', -1) < 0:
            return '<%s [closed]>' % title
        if getattr(self, '_repr_running', False):
            raise RuntimeError('reentrant call inside ' + title + '.__repr__')
        try:
            name = self.name
        except AttributeError:
            return '<%s fd=%d mode=%r closefd=%r>' % (
                title, self._fd, self.mode, self._closefd)
        self._repr_running = True
        try:
            return '<%s name=%r mode=%r closefd=%r>' % (
                title, name, self.mode, self._closefd)
        finally:
            self._repr_running = False

    @property
    def _finalizing(self):
        return self._finalizing_state

    @_finalizing.setter
    def _finalizing(self, value):
        if not isinstance(value, bool):
            raise TypeError('attribute value type must be bool')
        self._finalizing_state = value

    @_finalizing.deleter
    def _finalizing(self):
        raise TypeError("can't delete numeric/char attribute")

    @property
    def _blksize(self):
        import posix
        try:
            st = posix.fstat(self._fd)
        except OSError:
            return DEFAULT_BUFFER_SIZE
        blksize = getattr(st, 'st_blksize', 0)
        if not blksize:
            return DEFAULT_BUFFER_SIZE
        return blksize

    @property
    def closefd(self):
        return self._closefd

    @property
    def mode(self):
        if self._created:
            return 'xb+' if self._readable else 'xb'
        if self._appending:
            return 'ab+' if self._readable else 'ab'
        if self._readable:
            return 'rb+' if self._writable else 'rb'
        return 'wb'

    def fileno(self):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        return self._fd
    def close(self):
        if not self._closed:
            if _finalizer_os is not None:
                os = _finalizer_os
            else:
                import os
            # The base close flushes while the stream still reads as open,
            # then the descriptor goes; a flush error must not leave it open.
            try:
                _IOBase.close(self)
            finally:
                try:
                    if self._closefd and self._fd >= 0:
                        os.close(self._fd)
                finally:
                    self._fd = -1
    def readable(self):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        return self._readable
    def writable(self):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        return self._writable
    def seekable(self):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        try:
            self.seek(0, 1)
        except OSError:
            return False
        return True
    def isatty(self):
        import os
        return os.isatty(self.fileno())
    def read(self, size=-1):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        if not self._readable:
            raise UnsupportedOperation('File or stream is not readable.')
        if type(size) is not int or size > _MAXSIZE or size < _MINSIZE:
            size = _size(size)
        if size < 0:
            return self.readall()
        try:
            return _finalizer_os.read(self._fd, size)
        except BlockingIOError:
            return None
    def readall(self):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        if not self._readable:
            raise UnsupportedOperation('File or stream is not readable.')
        chunks = []
        while True:
            chunk = self.read(DEFAULT_BUFFER_SIZE)
            if chunk is None:
                return b''.join(chunks) if chunks else None
            if not chunk:
                return b''.join(chunks)
            chunks.append(chunk)
    def readinto(self, buffer):
        with _get_buffer(buffer) as view:
            if view.readonly:
                raise TypeError('readinto() argument must be read-write bytes-like object, not ' + type(buffer).__name__)
            data = self.read(view.nbytes)
            if data is None:
                return None
            view.cast('B')[:len(data)] = data
            return len(data)
    def write(self, buffer):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        if not self._writable:
            raise UnsupportedOperation('File or stream is not writable.')
        try:
            return _finalizer_os.write(self._fd, buffer if type(buffer) is bytes else _buffer(buffer))
        except BlockingIOError:
            return None
    def seek(self, offset, whence=0):
        if self._closed:
            raise ValueError('I/O operation on closed file.')
        return _finalizer_os.lseek(self._fd, offset if type(offset) is int and _MINSIZE <= offset <= _MAXSIZE else _ssize(offset), whence if type(whence) is int and -2147483648 <= whence <= 2147483647 else _whence(whence))
    def truncate(self, size=None):
        self._checkClosed()
        self._checkWritable()
        if size is None:
            size = self.tell()
        size = _ssize(size)
        if _finalizer_os is not None:
            os = _finalizer_os
        else:
            import os
        os.ftruncate(self._fd, size)
        return size

_FILEIO_READINTO = FileIO.readinto

class _CheckedRaw:
    # Buffered streams refill through readinto() so that every raw object's
    # length contract is checked in one place.  A file descriptor whose
    # readinto is still the FileIO one is the exception: read() there is a
    # single bounded system call, so the file road below reaches it
    # directly, while a subclass, class or instance that replaced readinto
    # keeps the validation road untouched.
    def __init__(self, raw):
        self.raw = raw
        self._owned = True
        self._plain = type(raw) is FileIO or type(raw) is BytesIO
    @property
    def closed(self):
        if self._plain:
            return not self._owned or self.raw._closed
        return not self._owned or self.raw.closed
    def close(self):
        if self._owned:
            return self.raw.close()
    def __getattr__(self, name):
        return getattr(self.raw, name)
    # What the buffered layer asks of its raw stream most often is
    # answered here directly rather than through __getattr__.
    def readable(self):
        return self.raw.readable()
    def writable(self):
        return self.raw.writable()
    def seekable(self):
        return self.raw.seekable()
    def seek(self, *args, **kwargs):
        return self.raw.seek(*args, **kwargs)
    def tell(self):
        return self.raw.tell()
    def fileno(self):
        return self.raw.fileno()
    def readinto(self, buffer):
        raw = self.raw
        plain = self._plain and 'readinto' not in raw.__dict__
        count = raw.readinto(buffer if plain and type(buffer) is bytearray else memoryview(buffer))
        if count is None:
            return None
        try:
            count = _index(count)
            if count > _MAXSIZE or count < _MINSIZE:
                raise ValueError('cannot fit int into an index-sized integer')
        except Exception as error:
            raise OSError('raw readinto() failed') from error
        if count < 0 or count > len(buffer):
            raise OSError('raw readinto() returned invalid length %d (should have been between 0 and %d)' % (count, len(buffer)))
        return count
    def read(self, size=-1):
        if self._plain and 'readinto' not in self.raw.__dict__:
            # Our own streams answer a read with bytes or None already,
            # unless a program put a readinto of its own on this one.
            return self.raw.read(-1 if size is None else size)
        if size is None or size < 0:
            chunk = self.raw.read()
            if chunk is not None and not isinstance(chunk, bytes):
                raise TypeError('read() should return bytes')
            return chunk
        raw = self.raw
        if (type(raw) is FileIO and type(raw).readinto is _FILEIO_READINTO
                and 'readinto' not in getattr(raw, '__dict__', ())):
            data = raw.read(size)
            if data is None:
                return None
            if not isinstance(data, bytes):
                raise TypeError('read() should return bytes')
            if len(data) > size:
                raise OSError('read() should return at most %d bytes' % size)
            return data
        data = bytearray(size)
        count = self.readinto(data)
        return None if count is None else bytes(data[:count])
    def readall(self):
        method = getattr(self.raw, 'readall', None)
        if method is not None:
            result = method()
            if result is not None and not isinstance(result, bytes):
                raise TypeError('readall() should return bytes')
            return result
        chunks = []
        while True:
            chunk = self.read()
            if not chunk:
                return b''.join(chunks) if chunks else chunk
            chunks.append(chunk)
    def write(self, data):
        count = self.raw.write(data)
        if count is None:
            return None
        count = _ssize(count)
        if count < 0 or count > len(data):
            raise OSError('raw write() returned invalid length %d (should have been between 0 and %d)' % (count, len(data)))
        return count

class _BufferedLock:
    # Guard buffered operations against recursive callbacks on this runtime.
    def __init__(self):
        self._held = False
    # Protect the ownership change against an interrupted enter method.
    def __enter__(self):
        if self._held:
            raise RuntimeError('reentrant call inside buffered I/O')
        try:
            self._held = True
            return self
        except BaseException:
            self._held = False
            raise
    # Release in one native store, before another Python statement can
    # deliver a pending signal and interrupt the ownership transition.
    __exit__ = lambda self, kind, value, tb: setattr(self, '_held', False) or False

_IMPLS = {}

def _buffered_impl(kind):
    # The reference's buffered classes with `raw` and `closed` as plain
    # attribute and one property call: the implementation asks for both
    # at every step.
    impl = _IMPLS.get(kind)
    if impl is None:
        _pyio = _text_runtime()
        base = getattr(_pyio, kind)
        class _Impl(base):
            raw = None
            def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
                base.__init__(self, raw, buffer_size)
                self.raw = raw
                held = vars(self)
                if '_read_lock' in held:
                    self._read_lock = _BufferedLock()
                if '_write_lock' in held:
                    self._write_lock = _BufferedLock()
            @property
            def closed(self):
                # The checked raw stream's own answer, without its call
                # for the streams it knows to be plain.
                raw = self.raw
                if raw is not None and raw._plain:
                    return not raw._owned or raw.raw._closed
                return raw.closed
            def detach(self):
                result = base.detach(self)
                self.raw = None
                return result
        if kind == 'BufferedReader':
            # Retain the consumed prefix of small refills for backward seeks.
            def _read_unlocked(self, size):
                result = base._read_unlocked(self, size)
                if size is not None and 0 < size <= self.buffer_size and result and self._read_pos == 0:
                    unread = len(self._read_buf)
                    self._read_buf = (result + self._read_buf)[-self.buffer_size:]
                    self._read_pos = len(self._read_buf) - unread
                return result
            _Impl._read_unlocked = _read_unlocked
        impl = _IMPLS[kind] = _Impl
    return impl

class BufferedReader(_BufferedIOBase):
    def _dealloc_warn(self, source):
        try:
            self.raw._dealloc_warn(source)
        except Exception:
            pass
    # The implementation while it is usable, read without a property call;
    # None whenever _impl has to say why not.
    _live = None
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        self._initialize(_buffered_impl('BufferedReader'), raw, buffer_size)
    def _initialize(self, factory, raw, size):
        previous = self.__dict__.get('_implementation')
        if previous is not None and previous.raw is not None:
            previous.raw._owned = False
        self._live = None
        self._implementation = None
        self._detached = False
        self._raw = raw
        size = _ssize(size)
        if size <= 0:
            raise ValueError('buffer size must be strictly positive')
        self._implementation = factory(_CheckedRaw(raw), size)
        self._live = self._implementation
    @property
    def _impl(self):
        held = self.__dict__
        result = held.get('_implementation')
        if result is not None and not held.get('_detached'):
            return result
        if getattr(self, '_detached', False):
            raise ValueError('raw stream has been detached')
        result = getattr(self, '_implementation', None)
        if result is None:
            raise ValueError('I/O operation on uninitialized object')
        return result
    @property
    def raw(self):
        return getattr(self, '_raw', None)
    def __getattr__(self, name):
        if name in ('_implementation', '_raw', '_repr_running', '_detached'):
            raise AttributeError(name)
        return getattr(self._impl, name)
    def _dealloc_warn(self, source):
        raw = self.raw
        if raw is not None:
            notify = getattr(raw, '_dealloc_warn', None)
            if notify is not None:
                notify(source)
    def __repr__(self):
        title = type(self).__module__ + '.' + type(self).__qualname__
        if getattr(self, '_repr_running', False):
            raise RuntimeError('reentrant call inside ' + title + '.__repr__')
        self._repr_running = True
        try:
            try:
                name = self.raw.name
            except AttributeError:
                return '<' + title + '>'
            return '<%s name=%r>' % (title, name)
        finally:
            self._repr_running = False
    def __getstate__(self):
        raise TypeError("cannot pickle '" + type(self).__name__ + "' instances")
    def readinto(self, buffer):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.readinto(buffer)
    def readinto1(self, buffer):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.readinto1(buffer)
    def truncate(self, pos=None):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.truncate(pos)
    def isatty(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.isatty()
    def fileno(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.fileno()
    def read(self, size=-1):
        impl = self._live
        if impl is None:
            impl = self._impl
        # A read the buffer already holds is answered here, the way the
        # reference's fast path answers it, without the lock and the
        # refill road: only for the plain reader, whose read does not
        # flush first.
        if type(self) is BufferedReader and type(size) is int and size >= 0 and not impl._read_lock._held:
            held = impl._read_buf
            at = impl._read_pos
            if size <= len(held) - at:
                impl._read_pos = at + size
                return held[at:at + size]
        return impl.read(size)
    def read1(self, size=-1):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.read1(size)
    def readline(self, size=-1):
        size = _size(size)
        self._checkClosed()
        return self._impl.readline(size)
    @property
    def closed(self):
        impl = self._live
        if impl is None:
            impl = self._impl
            return impl.closed
        # The checked raw's own answer, read without its property call.
        checked = impl.raw
        if not checked._owned:
            return True
        return checked.raw._closed if checked._plain else checked.raw.closed
    def _dealloc_warn(self, source):
        raw = getattr(self, '_raw', None)
        warn = getattr(raw, '_dealloc_warn', None)
        if warn is not None:
            warn(source)
    def close(self):
        if self.closed:
            return None
        try:
            try:
                if self._finalizing:
                    self._dealloc_warn(self)
            finally:
                self.flush()
        finally:
            self._impl.raw.close()
    def flush(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.flush()
    # The reference's reader answers readable and seekable with the raw
    # stream's own answer, so a usable stream asks the raw stream directly;
    # writable is the base answer unless the kind says otherwise.
    def readable(self):
        if self._live is None:
            return self._impl.readable()
        return self._raw.readable()
    def writable(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.writable()
    def seekable(self):
        if self._live is None:
            return self._impl.seekable()
        return self._raw.seekable()
    def seek(self, pos, whence=0):
        impl = self._live
        if impl is None:
            impl = self._impl
        # Keep buffered bytes when a read-only stream seeks within them.
        if type(self) is BufferedReader and type(pos) is int and type(whence) is int and whence in (0, 1):
            impl._checkClosed('seek of closed file')
            with impl._read_lock:
                if impl.raw.seekable():
                    try:
                        end = impl.raw.tell()
                    except (OSError, ValueError):
                        end = None
                    if type(end) is int and end >= 0:
                        start = end - len(impl._read_buf)
                        target = pos if whence == 0 else start + impl._read_pos + pos
                        if 0 <= start <= target <= end:
                            impl._read_pos = target - start
                            return target
        return impl.seek(pos, whence)
    def tell(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.tell()
    def detach(self):
        self.flush()
        checked = self._impl.detach()
        self._detached = True
        self._live = None
        self._raw = None
        return checked.raw
    def write(self, b):
        # The accelerated buffer consumes bytes from a buffer provider,
        # including arrays whose iteration yields wider numeric elements.
        kind = type(b)
        if kind is not bytes and kind is not bytearray and not isinstance(b, (bytes, bytearray)):
            b = memoryview(b).tobytes()
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.write(b)

class BufferedWriter(BufferedReader):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        self._initialize(_buffered_impl('BufferedWriter'), raw, buffer_size)
    # The reference's writer answers writable with the raw stream's own
    # answer and readable with the base answer.
    def readable(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.readable()
    def writable(self):
        if self._live is None:
            return self._impl.writable()
        return self._raw.writable()

class BufferedRandom(BufferedReader):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        self._initialize(_buffered_impl('BufferedRandom'), raw, buffer_size)
    writable = BufferedWriter.writable

class BufferedRWPair(BufferedReader):
    def __init__(self, reader, writer, buffer_size=DEFAULT_BUFFER_SIZE):
        _pyio = _text_runtime()
        self._implementation = _pyio.BufferedRWPair(reader, writer, buffer_size)
        self._live = self._implementation
    # A pair has no single checked raw stream: its implementation answers.
    def readable(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.readable()
    def writable(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.writable()
    def seekable(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.seekable()
    @property
    def closed(self):
        impl = self._live
        if impl is None:
            impl = self._impl
        return impl.closed
    # Forward resource warnings to each actual raw stream of the pair.
    def _dealloc_warn(self, source):
        implementation = getattr(self, '_implementation', None)
        for side in ('reader', 'writer'):
            stream = getattr(implementation, side, None)
            raw = getattr(stream, 'raw', None)
            warn = getattr(raw, '_dealloc_warn', None)
            if warn is not None:
                warn(source)
    def close(self):
        return self._impl.close()

# Mode strings open() has found well formed: the checks depend on the
# text of the mode alone.
_GOOD_MODES = set()

class _Open:
    __name__ = 'open'
    def __call__(self, file, mode='r', buffering=-1, encoding=None, errors=None, newline=None, closefd=True, opener=None):
        if type(mode) is not str or mode not in _GOOD_MODES:
            if not isinstance(mode, str):
                raise TypeError('open() argument 2 must be str, not ' + type(mode).__name__)
            letters = set(mode)
            if not letters <= set('axrwb+t') or len(mode) != len(letters):
                raise ValueError('invalid mode: ' + repr(mode))
            if 't' in letters and 'b' in letters:
                raise ValueError("can't have text and binary mode at once")
            if len(letters & set('axrw')) != 1:
                raise ValueError('Must have exactly one of create/read/write/append mode and at most one plus')
            if type(mode) is str:
                _GOOD_MODES.add(mode)
        if encoding is None and 'b' not in mode and sys.flags.warn_default_encoding:
            # The warning is aimed at the line that called open().
            encoding = text_encoding(None)
            if encoding == 'locale' and type(file) is str and opener is None and 'r' not in mode:
                encoding = None
        if type(file) is not str or opener is not None or encoding is not None or errors is not None or newline is not None or 'b' in mode or 'r' in mode:
            if type(file) is not str and not isinstance(file, int) and not hasattr(type(file), '__index__') and not isinstance(file, float):
                import os
                file = os.fspath(file)
            binary = 'b' in mode
            if binary and (encoding is not None or errors is not None or newline is not None):
                raise ValueError("binary mode doesn't take an encoding, errors, or newline argument")
            if binary and buffering == 1:
                import warnings
                warnings.warn("line buffering (buffering=1) isn't supported in "
                              "binary mode, the default buffer size will be used",
                              RuntimeWarning, 2)
            raw = FileIO(file, mode.replace('t', ''), closefd, opener)
            try:
                if buffering == 0:
                    if not binary:
                        raise ValueError("can't have unbuffered text I/O")
                    return raw
                size = DEFAULT_BUFFER_SIZE if buffering < 0 or buffering == 1 else _index(buffering)
                if '+' in mode:
                    buffer = BufferedRandom(raw, size)
                elif raw._readable:
                    buffer = BufferedReader(raw, size)
                else:
                    buffer = BufferedWriter(raw, size)
                if binary:
                    return buffer
                text = TextIOWrapper(buffer, encoding, errors, newline, line_buffering=buffering == 1)
                text.mode = mode
                return text
            except:
                raw.close()
                raise
        from builtins import _host_open
        return _host_open(file, mode, buffering, encoding, errors, newline, closefd, opener)

open = _Open()

def open_code(path):
    return open(path, 'rb')

import builtins as _builtins
_builtins.open = open

for _type in (BytesIO, StringIO, TextIOWrapper, IncrementalNewlineDecoder, BufferedReader, BufferedWriter, BufferedRandom, BufferedRWPair, FileIO):
    _type.__module__ = '_io'
