# Only an in-memory text stream is carried here.
class StringIO:
    def __init__(self, initial_value='', newline='\n'):
        self.text = initial_value
        self.position = 0
        self.closed = False

    def write(self, text):
        if self.closed:
            raise 'ValueError: I/O operation on closed file'
        self.text = self.text[:self.position] + text + self.text[self.position + len(text):]
        self.position += len(text)
        return len(text)

    def getvalue(self):
        return self.text

    def read(self, size=-1):
        if size < 0:
            size = len(self.text) - self.position
        value = self.text[self.position:self.position + size]
        self.position += len(value)
        return value

    def seek(self, offset, whence=0):
        if whence == 2:
            offset += len(self.text)
        elif whence == 1:
            offset += self.position
        self.position = offset
        return offset

    def tell(self):
        return self.position

    def flush(self):
        pass

    def close(self):
        self.closed = True

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
        return False
