import warnings
with warnings.catch_warnings(record=True) as w:
    warnings.filterwarnings('ignore', message='QUIET.*')
    warnings.warn('quiet words')
    warnings.simplefilter('module')
    warnings.warn('same')
    warnings.warn('same')
    warnings.simplefilter('default')
    for i in range(2):
        warnings.warn('line')
    warnings.warn('line')
print(len(w))
print(str(w[0].message), str(w[1].message), str(w[2].message))
