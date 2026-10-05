import io, weakref, gc
f = io.BytesIO(b'ab')
r = weakref.ref(f)
v = f.getbuffer()
print(type(v.obj).__name__, bytes(v))
del f
gc.collect()
print(r() is not None)
v.release()
gc.collect()
print(r() is None)
for args in ((11, 'wait'), (11, 'wait', 3), (11, 'wait', 'f'), (11, 'wait', -2)):
    e = BlockingIOError(*args)
    print(e.args, str(e), e.filename, getattr(e, 'characters_written', None))
for cls in (io.BufferedReader, io.BufferedRandom):
    f = cls(io.BytesIO(b'abcd'))
    out = bytearray(2)
    print(cls.__name__, f.readinto1(out), out, f.tell())
    f.close()
    try:
        f.readinto(out)
    except Exception as e:
        print(type(e).__name__)
