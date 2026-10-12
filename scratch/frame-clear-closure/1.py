import traceback
import unittest
def maker():
    value = object()
    def getter():
        return value
    try:
        raise ValueError('finished frame')
    except ValueError as error:
        return getter, error.__traceback__, value
getter, tb, value = maker()
frame = tb.tb_frame
code = frame.f_code
globals_book = frame.f_globals
traceback.clear_frames(tb)
assert frame.f_code is code
assert frame.f_globals is globals_book
assert frame.f_locals == {}
assert getter() is value
def still_running():
    value = object()
    def getter():
        return value
    with unittest.TestCase().assertRaises(ValueError):
        int('invalid')
    assert getter() is value
still_running()
print('frame clearing preserves live closure cells')
