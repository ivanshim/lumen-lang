# Native records contain kernel instructions; foreign CPython bytecode stays unsupported.
_archive_unmarshal = _unmarshal_code
def _unmarshal_code(self, pathname, fullpath, fullname, data):
    _bootstrap_external._classify_pyc(data, fullname, {'name': fullname, 'path': fullpath})
    if not data[16:].startswith(marshal._code_prefix):
        raise ZipImportError('CPython bytecode is not supported', name=fullname, path=pathname)
    return _archive_unmarshal(self, pathname, fullpath, fullname, data)
