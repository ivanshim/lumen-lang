import warnings
warnings.simplefilter("error", UserWarning)
try:
    warnings.warn("boom")
except UserWarning as e:
    print(e)
warnings.resetwarnings()
