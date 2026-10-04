import io

def outcome(label, function):
    try:
        print(label, repr(function()))
    except Exception as exc:
        print(label, type(exc).__name__, str(exc))

class Index:
    def __index__(self):
        return 2
class B(bytes):
    def __bytes__(self):
        return b'wrong'

def seek(pos, whence):
    f = io.BytesIO(b'abc')
    try:
        return f.seek(pos, whence), f.read()
    except Exception as e:
        return type(e).__name__, f.tell()
for pos, whence in [(-1,2), (-5,1),(-5,2),(-1,0),(0.5,0),(0,1.0),(Index(),0),(0,Index())]:
    outcome('seek', lambda: seek(pos, whence))
def snapshot():
    b = bytearray(b'abc')
    f = io.BytesIO(b)
    b[0] = 120
    return f.getvalue(), type(f.getvalue()).__name__
outcome('snapshot', snapshot)
for v in ['x', 3, B(b'abc'), memoryview(b'abcd')[::2]]:
    outcome('construct', lambda: io.BytesIO(v).getvalue())
    outcome('write', lambda: io.BytesIO().write(v))
def empty():
    f = io.BytesIO(b'a'); f.seek(3)
    return f.write(b''), f.tell(), f.getvalue()
outcome('empty', empty)
for n in [None, Index(), 1.5]:
    outcome('read-size', lambda: io.BytesIO(b'abc').read(n))
def invalid_read():
    f=io.BytesIO(b'abc');f.seek(1)
    try: f.read(1.5)
    except TypeError: pass
    return f.tell()
outcome('invalid-read-position', invalid_read)
def update():
    b = io.BytesIO(b'abc'); t = io.TextIOWrapper(b, encoding='ascii')
    t.seek(0); t.write('X'); t.flush(); p=t.tell(); t.seek(0)
    return p, t.read(), b.getvalue()
outcome('update', update)
class WriteOnly(io.BytesIO):
    def readable(self): return False
    def read(self, size=-1): raise AssertionError('read during construction')
def write_only():
    b=WriteOnly();t=io.TextIOWrapper(b,encoding='ascii'); t.write('X');t.flush()
    return t.readable(),t.writable(),b.getvalue()
outcome('write-only',write_only)
class ReadOnly(io.BytesIO):
    def writable(self): return False
outcome('read-only',lambda: io.TextIOWrapper(ReadOnly(b'a'),encoding='ascii').write('X'))
for encoding in ['utf-8','ascii']:
    for data, errors in [(b'\xffA\nB','ignore'),(b'\xc2A\nB','replace')]:
        def read_chars():
            f=io.TextIOWrapper(io.BytesIO(data),encoding=encoding,errors=errors)
            return f.read(1), f.read(1), f.read()
        outcome(encoding+'-'+errors+'-sized', read_chars)
        outcome(encoding+'-'+errors+'-line',lambda: io.TextIOWrapper(io.BytesIO(data),encoding=encoding,errors=errors).readline())
        outcome(encoding+'-'+errors+'-iter',lambda: list(io.TextIOWrapper(io.BytesIO(data),encoding=encoding,errors=errors)))
class Tiny(io.BytesIO):
    def read1(self, size=-1): return super().read1(1)
for errors in ['ignore','replace']:
    outcome('boundary-'+errors,lambda: io.TextIOWrapper(Tiny(b'\xc2A\nB'),encoding='utf-8',errors=errors).read())
for newline in [None,'','\n','\r','\r\n']:
    outcome('newlines',lambda: (io.StringIO('a\r\nb\rc\n',newline=newline).getvalue()))
