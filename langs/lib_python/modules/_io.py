# Runtime counterpart of CPython v3.14.8 Modules/_io (PSF License).
# Memory storage follows bytesio.c and stringio.c. Buffered text operations
# dispatch to the unchanged _pyio implementation after io finishes importing.
import operator
import sys
from builtins import _buffer_view

DEFAULT_BUFFER_SIZE = 128 * 1024
BlockingIOError = BlockingIOError

class UnsupportedOperation(OSError, ValueError):
    pass

def text_encoding(encoding, stacklevel=2):
    if encoding is None:
        encoding = 'utf-8' if sys.flags.utf8_mode else 'locale'
        if sys.flags.warn_default_encoding:
            import warnings
            warnings.warn("'encoding' argument not specified.", EncodingWarning, stacklevel + 1)
    return encoding

def _index(value):
    if not hasattr(type(value), '__index__'):
        raise TypeError("'" + type(value).__name__ + "' object cannot be interpreted as an integer")
    return operator.index(value)

def _size(value):
    if value is None:
        return -1
    if not hasattr(type(value), '__index__'):
        raise TypeError("argument should be integer or None, not '" + type(value).__name__ + "'")
    result = _index(value)
    if result > sys.maxsize or result < -sys.maxsize - 1:
        raise OverflowError("cannot fit '" + type(value).__name__ + "' into an index-sized integer")
    return result

def _ssize(value):
    value = _index(value)
    if value > sys.maxsize or value < -sys.maxsize - 1:
        raise OverflowError('Python int too large to convert to C ssize_t')
    return value

def _whence(value):
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
    with _get_buffer(value) as view:
        if not view.c_contiguous:
            raise BufferError('memoryview: underlying buffer is not C-contiguous')
        return view.tobytes()

class _IOBase:
    _closed = False
    def __del__(self):
        try:
            closed = self.closed
        except (AttributeError, ValueError):
            return
        if not closed:
            self.close()
    def _checkClosed(self):
        if self.closed:
            raise ValueError('I/O operation on closed file.')
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
        return None if n is None else bytes(buf[:n])
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
    def readinto(self, b):
        with memoryview(b) as view:
            target = view.cast('B')
            if target.readonly:
                raise TypeError('readinto() argument must be read-write bytes-like object')
            data = self.read(target.nbytes)
            if data is None:
                return None
            target[:len(data)] = data
            return len(data)
    def readinto1(self, b):
        return self.readinto(b)

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
        if self.closed:
            raise ValueError('I/O operation on closed file')
    def __init__(self, initial_bytes=None):
        if hasattr(self, '_data'):
            self._check_exports()
        self._data = bytearray() if initial_bytes is None else bytearray(_buffer(initial_bytes))
        self._pos = 0
        self._closed = False
    def _check_exports(self):
        if _export(self._data, True):
            raise BufferError('Existing exports of data: object cannot be re-sized')
    def getvalue(self):
        self._checkClosed()
        return bytes(self._data)
    def getbuffer(self):
        self._checkClosed()
        view = memoryview(self._data)
        view._object = _BytesIOBuffer(self)
        return view
    def close(self):
        if not self.closed:
            self._check_exports()
            self._data.clear()
            self._closed = True
    def readable(self):
        self._checkClosed()
        return True
    writable = readable
    seekable = readable
    def read(self, size=-1):
        size = _size(size)
        self._checkClosed()
        end = len(self._data) if size < 0 else min(len(self._data), self._pos + size)
        if end <= self._pos:
            return b''
        result = bytes(self._data[self._pos:end])
        self._pos = end
        return result
    read1 = read
    def readline(self, size=-1):
        size = _size(size)
        self._checkClosed()
        end = len(self._data) if size < 0 else min(len(self._data), self._pos + size)
        start = self._pos
        if end <= start:
            return b''
        data = bytes(self._data[start:end])
        at = data.find(b'\n')
        if at >= 0:
            data = data[:at + 1]
        self._pos += len(data)
        return data
    def write(self, b):
        with _get_buffer(b) as view:
            if not view.c_contiguous:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            self._checkClosed()
            self._check_exports()
            data = view.tobytes()
            n = len(data)
            if not n:
                return 0
            if self._pos > sys.maxsize - n:
                raise OverflowError('new position too large')
            if self._pos > len(self._data):
                self._data.extend(b'\0' * (self._pos - len(self._data)))
            self._data[self._pos:self._pos + n] = data
            self._pos += n
            return n
    def seek(self, pos, whence=0):
        pos = _ssize(pos)
        whence = _whence(whence)
        self._checkClosed()
        if whence == 0:
            if pos < 0:
                raise ValueError('negative seek value ' + str(pos))
        elif whence == 1:
            pos += self._pos
        elif whence == 2:
            pos += len(self._data)
        else:
            raise ValueError('invalid whence (' + str(whence) + ', should be 0, 1 or 2)')
        if pos > sys.maxsize:
            raise OverflowError('new position too large')
        self._pos = max(0, pos)
        return self._pos
    def tell(self):
        self._checkClosed()
        return self._pos
    def truncate(self, size=None):
        size = self._pos if size is None else _ssize(size)
        self._checkClosed()
        self._check_exports()
        if size < 0:
            raise ValueError('negative size value ' + str(size))
        del self._data[size:]
        return size
    def __getstate__(self):
        self._checkClosed()
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
    __slots__ = ('_text', '_pos', '_closed', '_newline', '_seen')
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
        self._pos = 0
        self._closed = False
        if initial_value:
            self.write(initial_value)
            self._pos = 0
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
        return self._text
    def readable(self):
        self._checkClosed()
        return True
    writable = readable
    seekable = readable
    def close(self):
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
        if self._pos > len(self._text):
            self._text += '\0' * (self._pos - len(self._text))
        self._text = self._text[:self._pos] + data + self._text[self._pos + len(data):]
        self._pos += len(data)
        return n
    def read(self, size=-1):
        size = _size(size)
        self._checkClosed()
        end = len(self._text) if size < 0 else min(len(self._text), self._pos + size)
        if end <= self._pos:
            return ''
        data = self._text[self._pos:end]
        self._pos = end
        return data
    def readline(self, size=-1):
        size = _size(size)
        self._checkClosed()
        data = self._text[self._pos:]
        end = len(data)
        if self._newline == '':
            for i in range(len(data)):
                if data[i] in ('\r', '\n'):
                    end = i + 1
                    if data[i:i + 2] == '\r\n':
                        end += 1
                    break
        else:
            nl = self._newline or '\n'
            i = data.find(nl)
            if i >= 0:
                end = i + len(nl)
        if size >= 0:
            end = min(end, size)
        self._pos += end
        return data[:end]
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
            pos = self._pos if whence == 1 else len(self._text)
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
        self._text = self._text[:size]
        return size
    def __getstate__(self):
        self._checkClosed()
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
        self._pos = pos
        if attrs is not None:
            self.__dict__.update(attrs)

# These bridge methods call the original Python algorithms with this object's
# state. Import is delayed because io declares the ABCs that _pyio registers.
def _method(instance, kind, name):
    import _pyio
    cls = getattr(_pyio, kind)
    member = getattr(cls, name)
    return member.__get__(instance, type(instance))

def _text_option(value, name):
    if value is not None:
        if not isinstance(value, str):
            raise TypeError(name + ' must be str or None, not ' + type(value).__name__)
        value.encode('utf-8')
    return value

class TextIOWrapper(_TextIOBase):
    _CHUNK_SIZE = 2048
    def __init__(self, buffer, encoding=None, errors=None, newline=None, line_buffering=False, write_through=False):
        import _pyio
        encoding = _text_option(encoding, 'encoding')
        errors = _text_option(errors, 'errors')
        if (encoding is not None and '\0' in encoding) or (errors is not None and '\0' in errors):
            raise ValueError('embedded null character')
        line_buffering = bool(_whence(line_buffering))
        write_through = bool(_whence(write_through))
        _pyio.TextIOWrapper.__init__(self, buffer, encoding, errors, newline, line_buffering, write_through)
    def __repr__(self):
        if self._buffer is None:
            return '<{}.{} encoding={!r}>'.format(type(self).__module__, type(self).__qualname__, self.encoding)
        return _method(self, 'TextIOWrapper', '__repr__')()
    def reconfigure(self, *, encoding=None, errors=None, newline=Ellipsis, line_buffering=None, write_through=None):
        encoding = _text_option(encoding, 'encoding')
        errors = _text_option(errors, 'errors')
        if encoding is not None and encoding != 'locale':
            import codecs
            codecs.lookup(encoding.split('\0', 1)[0])
        if line_buffering is not None:
            line_buffering = bool(_whence(line_buffering))
        if write_through is not None:
            write_through = bool(_whence(write_through))
        return _method(self, 'TextIOWrapper', 'reconfigure')(encoding=encoding, errors=errors, newline=newline, line_buffering=line_buffering, write_through=write_through)
    def _get_encoder(self):
        import codecs
        factory = codecs.getincrementalencoder(self._encoding.split('\0', 1)[0])
        self._encoder = factory(self._errors.split('\0', 1)[0])
        return self._encoder
    def _get_decoder(self):
        import codecs
        create = codecs.getincrementaldecoder(self._encoding.split('\0', 1)[0])
        decoded = create(self._errors.split('\0', 1)[0])
        if self._readuniversal:
            decoded = IncrementalNewlineDecoder(decoded, self._readtranslate)
        self._decoder = decoded
        return decoded
    def __getattr__(self, name):
        return _method(self, 'TextIOWrapper', name)
    @property
    def buffer(self):
        if self._buffer is None:
            raise ValueError('underlying buffer has been detached')
        return self._buffer
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
        return self._decoder.newlines if self._decoder else None
    @property
    def closed(self):
        return self.buffer.closed
    def read(self, size=-1):
        if not self.readable():
            raise UnsupportedOperation('not readable')
        return _method(self, 'TextIOWrapper', 'read')(size)
    def readline(self, size=-1):
        return _method(self, 'TextIOWrapper', 'readline')(size)
    def write(self, text):
        if not self.writable():
            raise UnsupportedOperation('not writable')
        return _method(self, 'TextIOWrapper', 'write')(text)
    def seek(self, cookie, whence=0):
        return _method(self, 'TextIOWrapper', 'seek')(cookie, whence)
    def tell(self):
        return _method(self, 'TextIOWrapper', 'tell')()
    def truncate(self, pos=None):
        return _method(self, 'TextIOWrapper', 'truncate')(pos)
    def readable(self):
        return self.buffer.readable()
    def writable(self):
        return self.buffer.writable()
    def seekable(self):
        return self.buffer.seekable()
    def flush(self):
        return _method(self, 'TextIOWrapper', 'flush')()
    def close(self):
        return _method(self, 'TextIOWrapper', 'close')()
    def detach(self):
        return _method(self, 'TextIOWrapper', 'detach')()
    def fileno(self):
        return self.buffer.fileno()
    def isatty(self):
        return self.buffer.isatty()
    def __next__(self):
        return _method(self, 'TextIOWrapper', '__next__')()

class IncrementalNewlineDecoder:
    def __init__(self, decoder, translate, errors='strict'):
        import _pyio
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

class FileIO(_RawIOBase):
    def __init__(self, file, mode='r', closefd=True, opener=None):
        raise UnsupportedOperation('File descriptors are not implemented')

class BufferedReader(_BufferedIOBase):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._impl = _pyio.BufferedReader(raw, buffer_size)
    def __getattr__(self, name):
        if name == '_impl':
            raise ValueError('I/O operation on uninitialized object')
        return getattr(self._impl, name)
    def readinto(self, buffer):
        return self._impl.readinto(buffer)
    def readinto1(self, buffer):
        return self._impl.readinto1(buffer)
    def truncate(self, pos=None):
        return self._impl.truncate(pos)
    def isatty(self):
        return self._impl.isatty()
    def fileno(self):
        return self._impl.fileno()
    def read(self, size=-1):
        return self._impl.read(size)
    def read1(self, size=-1):
        return self._impl.read1(size)
    @property
    def closed(self):
        return self._impl.closed
    def close(self):
        return self._impl.close()
    def flush(self):
        return self._impl.flush()
    def readable(self):
        return self._impl.readable()
    def writable(self):
        return self._impl.writable()
    def seekable(self):
        return self._impl.seekable()
    def seek(self, pos, whence=0):
        return self._impl.seek(pos, whence)
    def tell(self):
        return self._impl.tell()
    def detach(self):
        return self._impl.detach()
    def write(self, b):
        return self._impl.write(b)

class BufferedWriter(BufferedReader):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._impl = _pyio.BufferedWriter(raw, buffer_size)

class BufferedRandom(BufferedReader):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._impl = _pyio.BufferedRandom(raw, buffer_size)

class BufferedRWPair(BufferedReader):
    def __init__(self, reader, writer, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._impl = _pyio.BufferedRWPair(reader, writer, buffer_size)

class _Open:
    __name__ = 'open'
    def __call__(self, file, mode='r', buffering=-1, encoding=None, errors=None, newline=None, closefd=True, opener=None):
        from builtins import _host_open
        return _host_open(file, mode, buffering, encoding, errors, newline, closefd, opener)

open = _Open()

def open_code(path):
    return open(path, 'rb')

import builtins as _builtins
_builtins.open = open
