# The accelerator slot behind datetime.py.  This interpreter has no native
# datetime classes, so the classes here are the very objects _pydatetime
# defines (the same ones datetime exposes).  There is no C-API capsule:
# datetime_CAPI is deliberately absent.
from _pydatetime import *
from _pydatetime import __all__
