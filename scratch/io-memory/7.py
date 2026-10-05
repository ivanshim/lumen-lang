import io, _pyio, gc, weakref, sys, array
for module in (io, _pyio):
    events = []
    class Buffer(module.BytesIO):
        def close(self):
            events.append(self.getvalue())
            super().close()
    def exercise():
        class Text(module.TextIOWrapper):
            def __del__(self):
                events.append('del')
                finalizer = super().__del__
                finalizer()
        raw = Buffer()
        text = Text(raw, encoding='ascii')
        text.write('abc')
        del text
        gc.collect()
        print('finalize', events, raw.closed)
    exercise()
    observed = []
    old_hook = sys.unraisablehook
    sys.unraisablehook = lambda event: observed.append(type(event.exc_value).__name__)
    raw = module.BytesIO(b'data')
    view = raw.getbuffer()
    rw, vw = weakref.ref(raw), weakref.ref(view)
    cycle = [view]
    cycle.append(cycle)
    del raw, view, cycle
    gc.collect()
    sys.unraisablehook = old_hook
    print('collected', rw() is None, vw() is None, observed)
for bad in (-1, b'', 1000000):
    raw = io.BytesIO(b'abc')
    raw.readinto = lambda target: bad
    buffered = io.BufferedReader(raw, 8)
    try:
        buffered.read(1)
    except Exception as error:
        print('raw count', type(error).__name__, type(error.__cause__).__name__)
    buffered.close()
raw = io.BytesIO()
f = io.BufferedWriter(raw, 8)
f.__init__(raw, 16)
print('reinit', f.write(b'abc'), raw.closed)
f.flush()
print('written', raw.getvalue())
try:
    f.__init__(raw, 0)
except Exception as error:
    print('bad init', type(error).__name__)
try:
    f.write(b'x')
except Exception as error:
    print('invalidated', type(error).__name__)
try:
    f.raw = io.BytesIO()
except Exception as error:
    print('readonly', type(error).__name__)
data = array.array('i', b'x' * 32)
print('array', len(data), len(data.tobytes()))
data.frombytes(b'y' * 8)
print('frombytes', len(data), data.tobytes() == b'x' * 32 + b'y' * 8)
text = io.TextIOWrapper(io.BytesIO(), encoding='ascii')
text.detach()
print('detached', repr(text))

class NamedBytes(io.BytesIO):
    def read(self, size=-1):
        raise AssertionError('read override was called')
raw = NamedBytes(b'abc')
target = bytearray(3)
print('direct readinto', raw.readinto(target), bytes(target))
for cls in (io.BufferedReader, io.BufferedWriter, io.BufferedRandom):
    raw = NamedBytes()
    stream = cls(raw)
    raw.name = stream
    try:
        repr(stream)
    except Exception as error:
        print(cls.__name__, type(error).__name__, str(error))
    del raw.name
    print('detach', stream.detach() is raw)

signed = array.array('b', b'\x80\xff')
print('signed bytes', signed.tolist(), signed.tobytes(), memoryview(signed).tobytes())
text = io.TextIOWrapper(io.BytesIO(), encoding='ascii')
print('chunk size', text._CHUNK_SIZE)
text._CHUNK_SIZE = 17
try:
    text._CHUNK_SIZE = 0
except Exception as error:
    print('chunk validation', type(error).__name__, text._CHUNK_SIZE)
try:
    text.__init__(io.BytesIO(), newline='invalid')
except ValueError:
    try:
        text.read()
    except Exception as error:
        print('text invalidated', type(error).__name__)
