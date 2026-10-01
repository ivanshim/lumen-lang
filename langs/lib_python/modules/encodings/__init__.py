# Selected source from CPython commit 3b564385e4c9, Lib/encodings/__init__.py.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# Used under the PSF license in tests/python/LICENSE.
import encodings.aliases as aliases

def normalize_encoding(encoding):

    """ Normalize an encoding name.

        Normalization works as follows: all non-alphanumeric
        characters except the dot used for Python package names are
        collapsed and replaced with a single underscore, e.g. '  -;#'
        becomes '_'. Leading and trailing underscores are removed.

        Note that encoding names should be ASCII only.

    """
    if isinstance(encoding, bytes):
        encoding = str(encoding, "ascii")

    if not encoding.isascii():
        import warnings
        warnings.warn(
            "Support for non-ascii encoding names will be removed in 3.17",
            DeprecationWarning, stacklevel=2)

    return _normalize_encoding(encoding)

# _codecs implements this helper in C at the pinned revision.
def _normalize_encoding(name):
    pieces = []
    punctuation = False
    for character in name:
        if character == '.' or (character.isascii() and character.isalnum()):
            if punctuation and pieces:
                pieces.append('_')
            pieces.append(character)
            punctuation = False
        else:
            punctuation = True
    return ''.join(pieces)
