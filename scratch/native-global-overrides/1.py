original_len = len
space = globals()
for i in range(30):
    space['spare_' + str(i)] = i
space[42] = 'non-name key'
assert len([1, 2]) == 2
space['len'] = lambda value: 7
assert len([]) == 7
space['len'] = original_len
assert len([1, 2, 3]) == 3
space.pop('len')
assert len([1, 2]) == 2
space.update({'len': lambda value: 11})
assert len([]) == 11
del space['len']
assert len([]) == 0
assert space[42] == 'non-name key'
print('native globals observe live writes, replacement and deletion')
