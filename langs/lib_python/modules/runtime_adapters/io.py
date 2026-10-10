# Retain the implementation even when _pyio imports io during its own initialization.
import _io as _native_io
_runtime_pyio = _native_io._text_runtime()
if hasattr(_runtime_pyio, 'TextIOWrapper'):
    _native_io._install_text_helpers()
