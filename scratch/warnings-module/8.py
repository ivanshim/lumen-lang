import warnings
for category in [Warning, UserWarning, DeprecationWarning, SyntaxWarning, RuntimeWarning, FutureWarning, PendingDeprecationWarning, ImportWarning, UnicodeWarning, BytesWarning, ResourceWarning, EncodingWarning]:
    instance = category('word')
    print(category.__name__, isinstance(instance, Warning), isinstance(instance, Exception), str(instance))
class Special(UserWarning):
    pass
with warnings.catch_warnings(record=True) as records:
    warnings.simplefilter('always')
    warnings.warn(Special('custom'))
print(records[0].category.__name__, str(records[0].message))
