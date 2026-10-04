# From CPython v3.14.8 (8e6e75d9102e), Lib/sre_compile.py; PSF License.
import warnings
warnings.warn(f"module {__name__!r} is deprecated",
              DeprecationWarning,
              stacklevel=2)

from re import _compiler as _
globals().update({k: v for k, v in vars(_).items() if k[:2] != '__'})
