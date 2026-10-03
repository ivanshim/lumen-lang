class Storage:
    __slots__ = ('data', 'position')
    def __init__(self):
        self.data = bytearray(b'ab')
        self.position = 0
    def write(self):
        self.data[self.position:self.position + 1] = b'C'
        print(self.data, self.position)
a = Storage()
a.write()
b = Storage()
b.write()
a.write()
class Index:
    def __index__(self):
        return 4
for size in (Index(), 'a', 10**100, -1):
    data = bytearray(b'a')
    try:
        print(data.resize(size), data)
    except Exception as error:
        print(type(error).__name__, str(error))
class Buffer:
    def __buffer__(self, flags):
        raise TypeError('export failed')
import io
for operation in (io.BytesIO, io.BytesIO().write):
    try:
        operation(Buffer())
    except Exception as error:
        print(type(error).__name__, str(error))
