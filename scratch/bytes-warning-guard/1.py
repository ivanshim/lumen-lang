import sys
import warnings
raw = b'a'
text = 'a'
mutable = bytearray(raw)
with warnings.catch_warnings(record=True) as seen:
    warnings.simplefilter('always', BytesWarning)
    assert raw != text
    assert mutable != text
    assert raw != 1
    assert 1 == 1 and text == text and raw == raw
assert len(seen) == (3 if sys.flags.bytes_warning else 0)
print('mixed byte comparisons preserve warning behavior')
