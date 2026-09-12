import warnings
with warnings.catch_warnings(record=True) as w:
    warnings.filterwarnings("ignore", message="quiet")
    warnings.warn("quiet now")
    print(len(w))
    warnings.simplefilter("once")
    warnings.warn("same")
    warnings.warn("same")
    print(len(w), str(w[0].message))
