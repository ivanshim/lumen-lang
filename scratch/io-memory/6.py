import io, codecs
for value in ('utf-8\0', 'locale\0', '\udcfe'):
    for operation in ('lookup', 'construct', 'reconfigure'):
        try:
            if operation == 'lookup':
                codecs.lookup(value)
            elif operation == 'construct':
                io.TextIOWrapper(io.BytesIO(), encoding=value)
            else:
                io.TextIOWrapper(io.BytesIO(), encoding='ascii').reconfigure(encoding=value)
            print(repr(value), operation, 'OK')
        except Exception as error:
            print(repr(value), operation, type(error).__name__, str(error))
f = io.TextIOWrapper(io.BytesIO(), encoding='ascii')
f.detach()
print(repr(f).endswith(" encoding='ascii'>"))
b = io.BytesIO()
f = io.TextIOWrapper(b, encoding='ascii')
f.reconfigure(encoding='utf-8\0suffix')
print(repr(f.encoding), f.write('é'))
f.flush()
print(b.getvalue())
