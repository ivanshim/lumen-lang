# The byte-for-byte codec and the one spelling far characters as escapes,
# read from a row of bytes and written back to one.
print(repr(b'\x00\x7f\x80\xff'.decode('latin-1')))
print(repr(b'\xc3\xa9'.decode('iso-8859-1')))
print(repr(''.encode('latin-1')))
print(repr(chr(233).encode('latin-1')))
print(repr(b''.decode('raw-unicode-escape')))
print(repr(br'\u0663\u0661\u0664 '.decode('raw_unicode_escape')))
print(repr(br'\U0001f600'.decode('raw-unicode-escape')))
print(repr(br'\\u0041'.decode('raw-unicode-escape')))
print(repr(br'\n\x41'.decode('raw-unicode-escape')))
print(repr(chr(256).encode('raw-unicode-escape')))
print(repr(chr(0x1f600).encode('raw-unicode-escape')))
