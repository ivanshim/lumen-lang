import unittest
import traceback
case = unittest.TestCase()
def boom(*, message): raise ValueError(message)
assert case.assertRaises((ValueError, TypeError), boom, message='expected') is None
case.assertRaisesRegex(ValueError, 'expected', boom, message='expected')
with case.assertRaises(ValueError) as caught:
    boom(message='context')
assert str(caught.exception) == 'context'
try:
    case.assertRaises(ValueError, lambda: None)
except AssertionError as error:
    assert str(error) == 'ValueError not raised'
else: raise AssertionError('missing negative assertion')
unexpected = KeyError('unexpected')
def wrong(): raise unexpected
try:
    case.assertRaises(ValueError, wrong)
except KeyError as error:
    assert error is unexpected
    assert error.__traceback__ is not None
else: raise AssertionError('unexpected exception swallowed')
calls = []
original = traceback.clear_frames
def observe(tb):
    calls.append(tb)
    return original(tb)
traceback.clear_frames = observe
case.assertRaises(ValueError, boom, message='observed')
assert len(calls) == 1
traceback.clear_frames = original
print('callable and context exception assertions preserve behavior and cleanup hooks')
