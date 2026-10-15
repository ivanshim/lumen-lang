# Retain the implementation even when _pyio imports io during its own initialization.
import _io as _native_io
_runtime_pyio = _native_io._text_runtime()
