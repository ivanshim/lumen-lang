# A traceback cycle must be collected, but a retained traceback keeps locals.
import gc
import weakref

finished = []

class Item:
    def __init__(self, name):
        self.name = name
    def __del__(self):
        finished.append(self.name)
    def __getitem__(self, index):
        raise IndexError

class Catch:
    def __enter__(self):
        return self
    def __exit__(self, kind, value, traceback):
        self.exception = value
        return True

def exhaust(iterator):
    with Catch():
        next(iterator)

def unobserved():
    value = Item('cycle')
    reference = weakref.ref(value)
    exhaust(iter(value))
    return reference

reference = unobserved()
gc.collect()
print('cycle released', reference() is None, finished == ['cycle'])

def retained():
    global reference
    value = Item('retained')
    reference = weakref.ref(value)
    try:
        raise ValueError('saved')
    except ValueError as error:
        return error

error = retained()
gc.collect()
print('trace retains local', reference() is not None)
trace = error.__traceback__
print('frame intact', trace.tb_frame.f_code.co_name == 'retained',
      trace.tb_frame.f_locals['value'] is reference())
del error, trace
gc.collect()
print('trace released', reference() is None, finished == ['cycle', 'retained'])
