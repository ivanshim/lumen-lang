import copy
import pickle

def stop():
    raise ValueError('done')

for exception in (ValueError, (TypeError, ValueError)):
    original = iter(stop, stop_exception=exception)
    assert list(copy.copy(original)) == []
    assert list(copy.deepcopy(original)) == []
    for protocol in range(pickle.HIGHEST_PROTOCOL + 1):
        restored = pickle.loads(pickle.dumps(original, protocol))
        assert list(restored) == []
    assert list(original) == []
print('callable iterator stop exceptions survive copy and pickle')
