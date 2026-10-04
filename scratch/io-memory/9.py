import array
class Source:
    def __bytes__(self):
        return b'converted'
    def __iter__(self):
        return iter([120])
print('priority', bytes(Source()), bytearray(Source()))
class Static:
    @staticmethod
    def __bytes__():
        return b'static'
print('descriptor', bytes(Static()))
class Bad:
    def __bytes__(self):
        return 1
try:
    bytes(Bad())
except Exception as error:
    print('bad result', type(error).__name__, str(error))
words = array.array('i', b'abcdefgh')
view = memoryview(words)
print('word buffer', bytes(view))
print('strided buffer', bytes(memoryview(b'abcdef')[::2]))
