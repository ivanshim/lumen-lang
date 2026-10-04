import io, codecs
class Exporter:
    def __init__(self, stream):
        self.stream = stream
    def __buffer__(self, flags):
        print('acquire', flags)
        return memoryview(b'x')
    def __release_buffer__(self, view):
        print('release', self.stream.getvalue())
f = io.BytesIO(b'ab')
print('write', f.write(Exporter(f)), f.getvalue())
for encoding in ('utf-8','utf-8-sig','utf-16','utf-16-le','utf-32','ascii'):
    d = codecs.getincrementaldecoder(encoding)()
    e = codecs.getincrementalencoder(encoding)()
    print(encoding, d.getstate(), e.getstate())
    b = e.encode('a')
    print(b, d.decode(b), d.getstate(), e.getstate())
class B(bytes):
    def __getitem__(self, item):
        return 88
    def __len__(self):
        return 1
print('subclass', io.BytesIO(B(b'abc')).getvalue())
for cls in (io.BytesIO,io.StringIO):
    f = cls.__new__(cls)
    try:
        print(cls.__name__, f.closed, f.getvalue())
    except Exception as e:
        print(cls.__name__, type(e).__name__, str(e))
