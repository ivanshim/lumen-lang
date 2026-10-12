assert 'abc'.startswith('a')
assert 'abc'.endswith('c')
assert 'ABC'.lower() == 'abc'
assert 'a b'.split() == ['a', 'b']
items = [1, 2]
append = items.append
append(3)
assert items == [1, 2, 3]
assert items.count(2) == 1
assert (1, 2).index(2) == 1
assert range(5).count(3) == 1
assert {'a': 1}.get('a') == 1
assert (7).bit_count() == 3
assert (2.5).as_integer_ratio() == (5, 2)
for value, name in [('abc', 'append'), ((1, 2), 'append'), ([1], 'startswith')]:
    assert not hasattr(value, name)
print('native method rosters preserve receiver-specific methods')
