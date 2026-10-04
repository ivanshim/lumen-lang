import _pyio
large = bytearray(128 * 1024)
large[:3] = b'abc'
del large[3:]
print('tail', bytes(large))
for span in (slice(None, None, 2), slice(None, None, -2), slice(8, 2), slice(2, 8)):
    data = bytearray(b'abcdefghij')
    del data[span]
    print('slice', bytes(data))
class Raw(_pyio.RawIOBase):
    def __init__(self):
        self.parts = [b'abc', b'd', b'efg']
    def readinto(self, buf):
        if not self.parts:
            return 0
        data = self.parts.pop(0)
        buf[:len(data)] = data
        return len(data)
print('raw', Raw().readall())
exported = bytearray(b'abc')
view = memoryview(exported)
del exported[1:1]
print('empty exported', bytes(exported))
try:
    del exported[1:]
except BufferError as error:
    print(type(error).__name__, str(error))
view.release()
