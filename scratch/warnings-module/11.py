import warnings
with warnings.catch_warnings(record=True) as records:
    warnings.filterwarnings('ignore', module=r'__main__\Z', lineno=4)
    warnings.warn('hidden')
    warnings.warn('shown')
print(len(records), records[0].lineno, str(records[0].message))
