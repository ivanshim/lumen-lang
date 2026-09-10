import warnings
warnings.resetwarnings()
with warnings.catch_warnings(record=True) as outer:
    warnings.simplefilter('always')
    warnings.warn('outer')
    with warnings.catch_warnings(record=True) as inner:
        warnings.simplefilter('ignore')
        warnings.warn('hidden')
    with warnings.catch_warnings():
        warnings.warn('still outer')
    with warnings.catch_warnings(action='error'):
        try:
            warnings.warn('raised')
        except UserWarning as e:
            print(e)
    warnings.warn('last')
print(len(outer), len(inner))
print(str(outer[0].message), str(outer[1].message), str(outer[2].message))
print(outer[0].lineno, outer[0].filename[-4:])
print(len(warnings.filters))
