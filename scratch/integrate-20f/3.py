result = '%.*f' % (1000001, 1.5)
assert len(result) == 1000003
assert result.startswith('1.5000')
assert len(format(1.5, '#.1200g')) == 1201
assert format(float('nan'), '.1200e') == 'nan'
for precision in (2**31, -(2**31)-1, 2**1000):
    try:
        '%.*f' % (precision, 1.5)
    except OverflowError as error:
        assert 'too big for precision' in str(error)
    else:
        raise AssertionError('overflow not rejected')
try:
    '%.*f' % (1.5, 1.5)
except TypeError as error:
    assert 'format argument 1: * requires int, not float' in str(error)
else:
    raise AssertionError('precision type not checked')
print('format diagnostics and large precision preserved')
