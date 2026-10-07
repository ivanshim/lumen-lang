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
            try:
                warn = getattr(self, '_dealloc_warn', None)
                if warn is not None:
                    warn(self)
            finally:
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
    def readinto(self, buffer):
        with memoryview(buffer) as view:
            target = view.cast('B')
            if target.readonly:
                raise TypeError('readinto() argument must be read-write bytes-like object')
            data = BytesIO.read(self, target.nbytes)
            target[:len(data)] = data
            return len(data)
    readinto1 = readinto
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
        # Immutable exact bytes need no temporary export or snapshot.
        if type(b) is bytes:
            self._checkClosed()
            self._check_exports()
            n = len(b)
            if not n:
                return 0
            if self._pos > sys.maxsize - n:
                raise OverflowError('new position too large')
            if self._pos > len(self._data):
                self._data.extend(b'\0' * (self._pos - len(self._data)))
            self._data[self._pos:self._pos + n] = b
            self._pos += n
            return n
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
    @property
    def _CHUNK_SIZE(self):
        return getattr(self, '_chunk_size', 8192)
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
        import _pyio
        self._initialized = False
        encoding = _text_option(encoding, 'encoding')
        errors = _text_option(errors, 'errors')
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
        if name in ('_initialized', '_repr_running', '_chunk_size'):
            raise AttributeError(name)
        return _method(self, 'TextIOWrapper', name)
    @property
    def buffer(self):
        if not getattr(self, '_initialized', False):
            raise ValueError('I/O operation on uninitialized object')
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
        return self._decoder.newlines if self._decoder and self._readuniversal else None
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
        import os
        if getattr(self, '_fd', -1) >= 0:
            self.close()
        if not isinstance(mode, str):
            raise TypeError('mode must be a string')
        base = [letter for letter in mode if letter in 'rwax']
        if len(base) != 1 or any(letter not in 'rwax+b' for letter in mode) or mode.count('+') > 1 or mode.count('b') > 1:
            raise ValueError('Must have exactly one of create/read/write/append mode and at most one plus')
        self._readable = base[0] == 'r' or '+' in mode
        self._writable = base[0] != 'r' or '+' in mode
        self._closefd = bool(closefd)
        self._fd = -1
        self._closed = True
        self.mode = ('ab' if base[0] == 'a' else 'xb' if base[0] == 'x' else 'rb' if base[0] == 'r' else 'wb') + ('+' if '+' in mode else '')
        if isinstance(file, float):
            raise TypeError('integer argument expected, got float')
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
            try:
                file_mode = os.fstat(fd).st_mode
            except OSError as exc:
                if exc.errno == 9:
                    raise
            else:
                if file_mode & 61440 == 16384:
                    raise IsADirectoryError(21, 'Is a directory', file)
            self.name = file
            self._fd = fd
            self._closed = False
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
    def _dealloc_warn(self, source):
        if self._fd >= 0 and self._closefd and not self.closed:
            import warnings
            warnings.warn('unclosed file ' + repr(source), ResourceWarning,
                          stacklevel=2, source=self)
    @property
    def closefd(self):
        return self._closefd
    def fileno(self):
        self._checkClosed()
        return self._fd
    def close(self):
        if not self.closed:
            import os
            try:
                if self._closefd:
                    os.close(self._fd)
            finally:
                self._fd = -1
                self._closed = True
    def readable(self):
        self._checkClosed()
        return self._readable
    def writable(self):
        self._checkClosed()
        return self._writable
    def seekable(self):
        self._checkClosed()
        try:
            self.seek(0, 1)
        except OSError:
            return False
        return True
    def isatty(self):
        import os
        return os.isatty(self.fileno())
    def read(self, size=-1):
        self._checkClosed()
        self._checkReadable()
        size = _size(size)
        if size < 0:
            return self.readall()
        import os
        try:
            return os.read(self._fd, size)
        except BlockingIOError:
            return None
    def readall(self):
        self._checkClosed()
        self._checkReadable()
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
        self._checkClosed()
        self._checkWritable()
        import os
        try:
            return os.write(self._fd, _buffer(buffer))
        except BlockingIOError:
            return None
    def seek(self, offset, whence=0):
        self._checkClosed()
        import os
        return os.lseek(self._fd, _ssize(offset), _whence(whence))
    def truncate(self, size=None):
        self._checkClosed()
        self._checkWritable()
        if size is None:
            size = self.tell()
        size = _ssize(size)
        import os
        os.ftruncate(self._fd, size)
        return size

class _CheckedRaw:
    # Buffered C streams use raw.readinto(), even when raw also has read().
    # Keep validation here so all refill paths share the same contract.
    def __init__(self, raw):
        self.raw = raw
        self._owned = True
    @property
    def closed(self):
        return not self._owned or self.raw.closed
    def close(self):
        if self._owned:
            return self.raw.close()
    def __getattr__(self, name):
        return getattr(self.raw, name)
    def readinto(self, buffer):
        count = self.raw.readinto(memoryview(buffer))
        if count is None:
            return None
        try:
            count = _index(count)
            if count > sys.maxsize or count < -sys.maxsize - 1:
                raise ValueError('cannot fit int into an index-sized integer')
        except Exception as error:
            raise OSError('raw readinto() failed') from error
        if count < 0 or count > len(buffer):
            raise OSError('raw readinto() returned invalid length %d (should have been between 0 and %d)' % (count, len(buffer)))
        return count
    def read(self, size=-1):
        if size is None or size < 0:
            chunk = self.raw.read()
            if chunk is not None and not isinstance(chunk, bytes):
                raise TypeError('read() should return bytes')
            return chunk
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

class BufferedReader(_BufferedIOBase):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._initialize(_pyio.BufferedReader, raw, buffer_size)
    def _initialize(self, factory, raw, size):
        previous = getattr(self, '_implementation', None)
        if previous is not None and previous.raw is not None:
            previous.raw._owned = False
        self._implementation = None
        self._detached = False
        self._raw = raw
        size = _ssize(size)
        if size <= 0:
            raise ValueError('buffer size must be strictly positive')
        self._implementation = factory(_CheckedRaw(raw), size)
    @property
    def _impl(self):
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
    def readline(self, size=-1):
        size = _size(size)
        self._checkClosed()
        return self._impl.readline(size)
    @property
    def closed(self):
        return self._impl.closed
    def close(self):
        if self.closed:
            return None
        try:
            self.flush()
        finally:
            self._impl.raw.close()
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
        self.flush()
        checked = self._impl.detach()
        self._detached = True
        self._raw = None
        return checked.raw
    def write(self, b):
        # The accelerated buffer consumes bytes from a buffer provider,
        # including arrays whose iteration yields wider numeric elements.
        if not isinstance(b, (bytes, bytearray)):
            b = memoryview(b).tobytes()
        return self._impl.write(b)

class BufferedWriter(BufferedReader):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._initialize(_pyio.BufferedWriter, raw, buffer_size)

class BufferedRandom(BufferedReader):
    def __init__(self, raw, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._initialize(_pyio.BufferedRandom, raw, buffer_size)

class BufferedRWPair(BufferedReader):
    def __init__(self, reader, writer, buffer_size=DEFAULT_BUFFER_SIZE):
        import _pyio
        self._implementation = _pyio.BufferedRWPair(reader, writer, buffer_size)
    def close(self):
        return self._impl.close()

class _Open:
    __name__ = 'open'
    def __call__(self, file, mode='r', buffering=-1, encoding=None, errors=None, newline=None, closefd=True, opener=None):
        if type(file) is not str or opener is not None or encoding is not None or errors is not None or newline is not None or 'b' in mode:
            if not isinstance(file, int) and not hasattr(type(file), '__index__') and not isinstance(file, float):
                import os
                file = os.fspath(file)
            binary = 'b' in mode
            if binary and (encoding is not None or errors is not None or newline is not None):
                raise ValueError("binary mode doesn't take an encoding, errors, or newline argument")
            raw = FileIO(file, mode.replace('t', ''), closefd, opener)
            try:
                if buffering == 0:
                    if not binary:
                        raise ValueError("can't have unbuffered text I/O")
                    return raw
                size = DEFAULT_BUFFER_SIZE if buffering < 0 or buffering == 1 else _index(buffering)
                if '+' in mode:
                    buffer = BufferedRandom(raw, size)
                elif raw.readable():
                    buffer = BufferedReader(raw, size)
                else:
                    buffer = BufferedWriter(raw, size)
                if binary:
                    return buffer
                return TextIOWrapper(buffer, encoding, errors, newline, line_buffering=buffering == 1)
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
