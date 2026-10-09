# Validate headers and reject foreign bytecode, retaining the source fallback.
def _unmarshal_code(self, pathname, fullpath, fullname, data):
    _bootstrap_external._classify_pyc(data, fullname, {'name': fullname, 'path': fullpath})
    raise ZipImportError('CPython bytecode is not supported', name=fullname, path=pathname)
