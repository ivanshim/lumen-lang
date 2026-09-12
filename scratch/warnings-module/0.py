import warnings
with warnings.catch_warnings(record=True) as w:
    warnings.simplefilter("always")
    warnings.warn("x", DeprecationWarning)
print(len(w), w[0].category.__name__, str(w[0].message))
