from collections import UserString
for text in ('abcabc', 'éa😀éa😀', '', 'abc' * 10000):
    wrapped = UserString(text)
    for needle in ('', 'a', 'é', '😀', 'missing'):
        for low, high in ((0, len(text)), (1, -1), (-4, 100000), (100001, 2), (-100000, 2)):
            for name in ('find', 'rfind', 'count', 'startswith', 'endswith'):
                assert getattr(wrapped, name)(needle, low, high) == getattr(text, name)(needle, low, high)
            for name in ('index', 'rindex'):
                try:
                    expected = getattr(text, name)(needle, low, high)
                except ValueError:
                    try:
                        getattr(wrapped, name)(needle, low, high)
                    except ValueError:
                        pass
                    else:
                        raise AssertionError('missing substring accepted')
                else:
                    assert getattr(wrapped, name)(needle, low, high) == expected
print('borrowed searches preserve bounds and Unicode offsets')
