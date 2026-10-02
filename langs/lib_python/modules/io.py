# Only an in-memory text stream is carried here.
class StringIO:
    def __init__(self, initial_value='', newline='\n'):
        if newline != "\n":
            raise 'NotImplementedError: alternate newline modes are not supported'
        self.text = initial_value
        self.position = 0
        self.closed = False

    def write(self, text):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        while len(self.text) < self.position:
            self.text += "\0"
        self.text = self.text[:self.position] + text + self.text[self.position + len(text):]
        self.position += len(text)
        return len(text)

    def getvalue(self):
        self._check()
        return self.text

    def read(self, size=-1):
        self._check()
        if size < 0:
            size = len(self.text) - self.position
        value = self.text[self.position:self.position + size]
        self.position += len(value)
        return value

    def seek(self, offset, whence=0):
        self._check()
        if whence not in [0, 1, 2]:
            raise 'ValueError: invalid whence'
        if whence != 0 and offset != 0:
            raise 'OSError: cannot do nonzero cur-relative seeks'
        if whence == 2:
            offset += len(self.text)
        elif whence == 1:
            offset += self.position
        if offset < 0:
            raise 'ValueError: negative seek position'
        self.position = offset
        return offset

    def tell(self):
        self._check()
        return self.position

    def flush(self):
        self._check()

    def close(self):
        self.closed = True

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
        return False

    def readline(self, size=-1):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        result = ''
        while self.position < len(self.text) and (size < 0 or len(result) < size):
            letter = self.read(1)
            result += letter
            if letter == '\n':
                break
        return result


    def readlines(self, hint=-1):
        self._check()
        lines = []
        length = 0
        while True:
            line = self.readline()
            if line == '':
                return lines
            lines = [*lines, line]
            length += len(line)
            if hint > 0 and length > hint:
                return lines

    def __iter__(self):
        self._check()
        return self

    def _line_more(self):
        self._line = self.readline()
        return self._line != ''

    def _line_value(self):
        return self._line

    def __next__(self):
        line = self.readline()
        if line == '':
            raise StopIteration
        return line

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')


# Native text-stream adapter. The binary buffer owns the contents and cursor.
class UnsupportedOperation(OSError, ValueError):
    pass


class TextIOWrapper:
    def __init__(self, buffer, encoding=None, errors=None, newline=None,
                 line_buffering=False, write_through=False):
        if newline not in (None, '', '\n', '\r', '\r\n'):
            raise ValueError('illegal newline value: ' + repr(newline))
        self.buffer = buffer
        self.encoding = encoding or 'utf-8'
        self.errors = errors or 'strict'
        self.line_buffering = bool(line_buffering)
        self.write_through = bool(write_through)
        self._newline = newline
        self._pending = b''
        self._telling = True

    @property
    def closed(self):
        return self.buffer.closed

    @property
    def name(self):
        return self.buffer.name

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file.')

    def readable(self):
        self._check()
        return self.buffer.readable()

    def writable(self):
        self._check()
        return self.buffer.writable()

    def seekable(self):
        self._check()
        return self.buffer.seekable()

    def _byte(self):
        if not self._pending:
            self._pending = self.buffer.read(2048)
        byte = self._pending[:1]
        self._pending = self._pending[1:]
        return byte

    def _character(self):
        data = self._byte()
        if not data:
            return ''
        encoding = self.encoding.lower().replace('_', '-')
        if encoding in ('utf-8', 'utf8'):
            first = data[0]
            width = 1
            if 194 <= first <= 223:
                width = 2
            elif 224 <= first <= 239:
                width = 3
            elif 240 <= first <= 244:
                width = 4
            while len(data) < width:
                byte = self._byte()
                if not byte:
                    break
                data += byte
        elif encoding not in ('ascii', 'us-ascii', 'latin-1', 'latin1', 'iso-8859-1'):
            raise UnsupportedOperation('sized reads require UTF-8 or a single-byte encoding')
        return data.decode(self.encoding, self.errors)

    def _translated_character(self):
        char = self._character()
        if char == '\r' and self._newline is None:
            byte = self._byte()
            if byte and byte != b'\n':
                self._pending = byte + self._pending
            return '\n'
        return char

    def read(self, size=-1):
        self._check()
        if not self.readable():
            raise UnsupportedOperation('not readable')
        if size is None or size < 0:
            data = self._pending + self.buffer.read()
            self._pending = b''
            text = data.decode(self.encoding, self.errors)
            if self._newline is None:
                text = text.replace('\r\n', '\n').replace('\r', '\n')
            return text
        result = ''
        while len(result) < size:
            char = self._translated_character()
            if not char:
                break
            result += char
        return result

    def readline(self, size=-1):
        self._check()
        if not self.readable():
            raise UnsupportedOperation('not readable')
        result = ''
        while size is None or size < 0 or len(result) < size:
            char = self._translated_character()
            if not char:
                break
            result += char
            ending = self._newline
            if ending is None and char == '\n':
                break
            if ending == '' and char in ('\r', '\n'):
                if char == '\r' and (size is None or size < 0 or len(result) < size):
                    byte = self._byte()
                    if byte == b'\n':
                        result += '\n'
                    elif byte:
                        self._pending = byte + self._pending
                break
            if ending and result.endswith(ending):
                break
        return result

    def write(self, text):
        self._check()
        if not self.writable():
            raise UnsupportedOperation('not writable')
        if not isinstance(text, str):
            raise TypeError("write() argument must be str, not " + type(text).__name__)
        length = len(text)
        if self._newline not in (None, '', '\n'):
            text = text.replace('\n', self._newline)
        self.buffer.write(text.encode(self.encoding, self.errors))
        self._pending = b''
        if self.write_through or self.line_buffering and ('\n' in text or '\r' in text):
            self.flush()
        return length

    def tell(self):
        self._check()
        if not self.seekable():
            raise UnsupportedOperation('underlying stream is not seekable')
        if not self._telling:
            raise OSError('telling position disabled by next() call')
        self.flush()
        return self.buffer.tell() - len(self._pending)

    def seek(self, offset, whence=0):
        self._check()
        if not self.seekable():
            raise UnsupportedOperation('underlying stream is not seekable')
        if whence not in (0, 1, 2):
            raise ValueError('invalid whence (' + str(whence) + ', should be 0, 1 or 2)')
        if whence == 1 and offset != 0:
            raise UnsupportedOperation("can't do nonzero cur-relative seeks")
        if whence == 2 and offset != 0:
            raise UnsupportedOperation("can't do nonzero end-relative seeks")
        if whence == 0 and offset < 0:
            raise ValueError('negative seek position ' + str(offset))
        if whence == 1:
            offset = self.tell()
            whence = 0
        self.flush()
        self._pending = b''
        return self.buffer.seek(offset, whence)

    def flush(self):
        self._check()
        self.buffer.flush()
        self._telling = True

    def close(self):
        if not self.closed:
            try:
                self.flush()
            finally:
                self.buffer.close()

    def readlines(self, hint=-1):
        lines = []
        total = 0
        for line in self:
            lines.append(line)
            total += len(line)
            if hint is not None and hint > 0 and total > hint:
                break
        return lines

    def writelines(self, lines):
        self._check()
        for line in lines:
            self.write(line)

    def __iter__(self):
        self._check()
        return self

    def __next__(self):
        self._telling = False
        line = self.readline()
        if line == '':
            self._telling = True
            raise StopIteration
        return line

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
        return False
