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
