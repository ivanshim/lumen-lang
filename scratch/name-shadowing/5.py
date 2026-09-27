import pickle
class MyEnum(enumerate):
    pass
round_trips = 0
for protocol in range(pickle.HIGHEST_PROTOCOL + 1):
    for width in range(1, 25):
        values = []
        for i in range(width):
            item = MyEnum('abcd', i)
            next(item)
            values.append(item)
        data = pickle.dumps(values, protocol)
        restored = pickle.loads(data)
        for i in range(width):
            assert type(restored[i]) is MyEnum
            assert list(restored[i]) == [(i + 1, 'b'), (i + 2, 'c'), (i + 3, 'd')]
        round_trips += 1
print('round trips', round_trips)

for protocol in range(pickle.HIGHEST_PROTOCOL + 1):
    shared = [1, 2]
    restored = pickle.loads(pickle.dumps([shared, shared], protocol))
    assert restored[0] is restored[1]
    cycle = []
    cycle.append(cycle)
    restored = pickle.loads(pickle.dumps(cycle, protocol))
    assert restored[0] is restored
print('memo aliases preserved')
