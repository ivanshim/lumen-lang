import warnings
def notify():
    warnings.warn('above', stacklevel=2)
with warnings.catch_warnings(record=True) as records:
    warnings.simplefilter('always')
    notify()
print(len(records), records[0].lineno, str(records[0].message))
