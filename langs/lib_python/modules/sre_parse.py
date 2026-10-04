# From CPython v3.14.8 (8e6e75d9102e), Lib/sre_parse.py; PSF License.
import warnings
warnings.warn(f"module {__name__!r} is deprecated",
              DeprecationWarning,
              stacklevel=2)

from re import _parser as _
globals().update({k: v for k, v in vars(_).items() if k[:2] != '__'})
