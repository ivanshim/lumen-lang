import gc
import weakref

class Item:
    pass

def cleanup():
    item = Item()
    reference = weakref.ref(item)
    def raise_error():
        local = item
        raise ValueError(item)
    try:
        raise_error()
    except ValueError as error:
        pass
    item = None
    gc.collect()
    print('except binding cleared', reference() is None)

cleanup()

def body(value):
    yield

item = Item()
reference = weakref.ref(item)
generator = body(item)
next(generator)
item = None
generator.close()
gc.collect()
print('closed frame released', generator.gi_frame is None, reference() is None)

item = Item()
reference = weakref.ref(item)
generator = body(item)
next(generator)
frame = generator.gi_frame
item = None
generator.close()
gc.collect()
print('retained frame intact', generator.gi_frame is None,
      frame.f_code.co_name == 'body', frame.f_locals['value'] is reference())
del frame
gc.collect()
print('retained frame released', reference() is None)
