import sys
import unittest
assert 'traceback' not in sys.modules
case = unittest.TestCase()
marker = object()
def boom():
    local = marker
    raise ValueError('expected')
case.assertRaises(ValueError, boom)
with case.assertRaises(ValueError) as captured:
    boom()
assert captured.exception.__traceback__ is None
assert marker is not None
assert 'traceback' not in sys.modules
cleared = []
class Frame:
    def clear(self):
        cleared.append(True)
class Trace:
    tb_frame = Frame()
    tb_next = None
unittest._clear_exception_frames(Trace())
assert cleared == [True]
print('native assertion cleanup avoids imports and retains the Python fallback')
