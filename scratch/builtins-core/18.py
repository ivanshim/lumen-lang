# The text reader given a row of bytes with an encoding, and given one
# without, where the row is spelled out rather than decoded.
print(repr(str(b'abc')))
print(repr(str(bytearray(b'hi'))))
print(repr(str(b'abc', 'utf-8')))
print(repr(str(b'abc', 'UTF_8')))
print(repr(str(b'abc', 'ascii', 'strict')))
print(repr(str(b'abc', encoding='us-ascii')))
print(repr(str(b'abc', errors='strict')))
print(repr(str(b'\xff', 'latin-1')))
print(repr(str(bytearray(b'\x80'), 'latin1')))
print(repr(str(br'\u0663\u0661\u0664 ', 'raw-unicode-escape')))
