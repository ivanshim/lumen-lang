#!/usr/bin/env python3
# Keep surrogate splitting on Python's complete whitespace rules for direct and saved calls.
for gap in ['\x1c', '\x1d', '\x1e', '\x1f']:
    text = gap + '\ud800' + gap + 'X' + gap
    saved_split = text.split
    saved_rsplit = text.rsplit
    expected = ['\ud800', 'X']
    assert text.split() == saved_split() == expected
    assert text.rsplit() == saved_rsplit() == expected
    assert text.split(maxsplit=0) == ['\ud800' + gap + 'X' + gap]
    assert saved_rsplit(sep=None, maxsplit=0) == [gap + '\ud800' + gap + 'X']
    assert text.split(None, 1) == ['\ud800', 'X' + gap]
    assert saved_rsplit(None, 1) == [gap + '\ud800', 'X']
    assert saved_split(gap, 1) == ['', '\ud800' + gap + 'X' + gap]
    assert text.rsplit(gap, 1) == [gap + '\ud800' + gap + 'X', '']
print('surrogate split and rsplit preserve control whitespace')
