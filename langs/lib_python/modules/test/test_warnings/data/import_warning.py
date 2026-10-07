# Source: CPython v3.14.8 / 8e6e75d9102e, Lib/test/test_warnings/data/import_warning.py; PSF License.
import warnings

warnings.warn('module-level warning', DeprecationWarning, stacklevel=2)
