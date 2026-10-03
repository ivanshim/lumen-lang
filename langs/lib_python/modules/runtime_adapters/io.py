# Runtime adapter: in-memory binary streams and a text wrapper, after
# CPython Modules/_io/bytesio.c and Modules/_io/textio.c at v3.14.8 /
# 8e6e75d9102e; PSF License.  Appended to io.py, which carries StringIO.


class BytesIO:
    def __init__(self, initial_bytes=b''):
        self._buffer = bytes(initial_bytes)
        self._pos = 0
        self._closed = False

    def _check(self):
        if self._closed:
            raise ValueError('I/O operation on closed file')

    def getvalue(self):
        self._check()
        return self._buffer

    def read(self, size=-1):
        self._check()
        if size is None or size < 0:
            size = len(self._buffer) - self._pos
        value = self._buffer[self._pos:self._pos + size]
        self._pos += len(value)
        return value

    def read1(self, size=-1):
        return self.read(size)

    def readline(self, size=-1):
        self._check()
        nl = self._buffer.find(b'\n', self._pos)
        if nl == -1:
            end = len(self._buffer)
        else:
            end = nl + 1
        if size is not None and size >= 0:
            end = min(end, self._pos + size)
        value = self._buffer[self._pos:end]
        self._pos = end
        return value

    def readlines(self, hint=-1):
        self._check()
        lines = []
        total = 0
        while True:
            line = self.readline()
            if not line:
                break
            lines.append(line)
            total += len(line)
            if hint > 0 and total >= hint:
                break
        return lines

    def write(self, data):
        self._check()
        data = bytes(data)
        head = self._buffer[:self._pos]
        tail = self._buffer[self._pos + len(data):]
        self._buffer = head + data + tail
        self._pos += len(data)
        return len(data)

    def seek(self, offset, whence=0):
        self._check()
        if whence == 1:
            offset += self._pos
        elif whence == 2:
            offset += len(self._buffer)
        elif whence != 0:
            raise ValueError('invalid whence')
        if offset < 0:
            raise ValueError('negative seek position')
        self._pos = offset
        return offset

    def tell(self):
        self._check()
        return self._pos

    def truncate(self, size=None):
        self._check()
        if size is None:
            size = self._pos
        self._buffer = self._buffer[:size]
        if self._pos > size:
            self._pos = size
        return size

    def flush(self):
        self._check()

    def close(self):
        self._closed = True

    @property
    def closed(self):
        return self._closed

    def readable(self):
        return True

    def writable(self):
        return True

    def seekable(self):
        return True

    def __iter__(self):
        return self

    def __next__(self):
        line = self.readline()
        if not line:
            raise StopIteration
        return line

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()


class TextIOWrapper:
    # The text is decoded from the buffer on first use; newline=None gives
    # universal-newline translation, as the C wrapper does.
    def __init__(self, buffer, encoding=None, errors=None, newline=None,
                 line_buffering=False, write_through=False):
        self._buffer = buffer
        if encoding is None:
            encoding = 'utf-8'
        self._encoding = encoding
        self._errors = errors or 'strict'
        self._newline = newline
        self.line_buffering = line_buffering
        self.write_through = write_through
        self._text = None
        self._pos = 0

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')

    def _load(self):
        if self._text is None:
            data = self._buffer.read()
            text = data.decode(self._encoding, self._errors)
            if self._newline is None:
                text = text.replace('\r\n', '\n').replace('\r', '\n')
            self._text = text

    def read(self, size=-1):
        self._check()
        self._load()
        if size is None or size < 0:
            size = len(self._text) - self._pos
        value = self._text[self._pos:self._pos + size]
        self._pos += len(value)
        return value

    def readline(self, size=-1):
        self._check()
        self._load()
        nl = self._text.find('\n', self._pos)
        if nl == -1:
            end = len(self._text)
        else:
            end = nl + 1
        if size is not None and size >= 0:
            end = min(end, self._pos + size)
        value = self._text[self._pos:end]
        self._pos = end
        return value

    def readlines(self, hint=-1):
        self._check()
        lines = []
        total = 0
        while True:
            line = self.readline()
            if not line:
                break
            lines.append(line)
            total += len(line)
            if hint > 0 and total >= hint:
                break
        return lines

    def write(self, text):
        self._check()
        data = text.encode(self._encoding, self._errors)
        return self._buffer.write(data)

    def seek(self, offset, whence=0):
        self._check()
        if whence != 0 or offset != 0:
            raise OSError('cannot do nonzero cur-relative seeks')
        self._load()
        self._pos = offset
        return offset

    def tell(self):
        self._check()
        return self._pos

    def flush(self):
        self._check()
        self._buffer.flush()

    def close(self):
        if not self.closed:
            self._buffer.close()

    @property
    def closed(self):
        return self._buffer.closed

    def readable(self):
        return True

    def writable(self):
        return True

    def seekable(self):
        return True

    def __iter__(self):
        return self

    def __next__(self):
        line = self.readline()
        if not line:
            raise StopIteration
        return line

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
