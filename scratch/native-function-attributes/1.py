import gc
import weakref

def factory(value):
    def function():
        return value
    return function

left = factory(1)
right = factory(2)
assert not hasattr(left, 'marker')
left.marker = 'left'
right.marker = 'right'
for _ in range(20):
    assert left.marker == 'left' and right.marker == 'right'
    assert left() == 1 and right() == 2
    assert 'marker' in dir(left)
left.__dict__['marker'] = 'updated'
assert left.marker == 'updated'
del left.marker
assert not hasattr(left, 'marker') and right.marker == 'right'
reference = weakref.ref(left)
del left
gc.collect()
assert reference() is None and right.marker == 'right'
for value in range(80):
    temporary = factory(value)
    assert not hasattr(temporary, 'marker')
    temporary.marker = value
    assert temporary.marker == value and temporary() == value
    if value % 10 == 0:
        gc.collect()
assert right.marker == 'right'
print('function attributes preserve identity, live dictionaries and collection')
