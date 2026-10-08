"""Native codec helpers used by the Python encoding registry."""
def _normalize_encoding(encoding):
    if not isinstance(encoding, str):
        raise TypeError('encoding must be a string')
    result = []
    separator = False
    for character in encoding:
        if character.isalnum() or character == '.':
            if separator and result:
                result.append('_')
            result.append(character)
            separator = False
        else:
            separator = True
    return ''.join(result)


_BUILTIN_ERROR_HANDLERS = (
    'strict', 'ignore', 'replace', 'xmlcharrefreplace',
    'backslashreplace', 'namereplace', 'surrogatepass', 'surrogateescape',
)

def _unregister_error(name):
    if not isinstance(name, str):
        raise TypeError('errors must be str')
    if name in _BUILTIN_ERROR_HANDLERS:
        raise ValueError("cannot un-register built-in error handler '%s'" % name)
    import codecs
    if name not in codecs._errors:
        return False
    del codecs._errors[name]
    return True
