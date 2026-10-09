# CPython's frozen bootstrap module, under the name its startup gives
# it. This runtime carries that machinery as importlib._bootstrap, so
# the frozen name answers as the very same module; the setup the
# reference runs while starting up is run here the first time the name
# is asked for.
import sys
import _imp
import importlib._bootstrap as _bootstrap
if not hasattr(_bootstrap, "_imp"):
    _bootstrap._setup(sys, _imp)
sys.modules[__name__] = _bootstrap
