# Stub: the plain C locale is the only locale carried here.

class Error(Exception):
    pass

LC_CTYPE = 0
LC_NUMERIC = 1
LC_TIME = 2
LC_COLLATE = 3
LC_MONETARY = 4
LC_MESSAGES = 5
LC_ALL = 6
CHAR_MAX = 127

# The names the C library accepts for the portable locale.
_C_LOCALE_NAMES = ('', 'C', 'POSIX')

def _known_category(category):
    return isinstance(category, int) and 0 <= category <= LC_ALL

def setlocale(category, locale=None):
    if not _known_category(category):
        raise Error('invalid locale category')
    if locale is None:
        return 'C'
    if locale in _C_LOCALE_NAMES:
        return 'C'
    raise Error('unsupported locale setting')

def getlocale(category=LC_CTYPE):
    if category == LC_ALL:
        raise TypeError('category LC_ALL is not supported')
    if not _known_category(category):
        raise Error('invalid locale category')
    return (None, None)

def localeconv():
    return {'decimal_point': '.', 'thousands_sep': '', 'grouping': [], 'int_curr_symbol': '', 'currency_symbol': '', 'mon_decimal_point': '', 'mon_thousands_sep': '', 'mon_grouping': [], 'positive_sign': '', 'negative_sign': '', 'int_frac_digits': 127, 'frac_digits': 127, 'p_cs_precedes': 127, 'p_sep_by_space': 127, 'n_cs_precedes': 127, 'n_sep_by_space': 127, 'p_sign_posn': 127, 'n_sign_posn': 127}

def strcoll(a, b):
    if not isinstance(a, str):
        raise TypeError('strcoll() argument 1 must be str')
    if not isinstance(b, str):
        raise TypeError('strcoll() argument 2 must be str')
    if '\0' in a or '\0' in b:
        raise ValueError('embedded null character')
    return (a > b) - (a < b)

def strxfrm(s):
    if not isinstance(s, str):
        raise TypeError('strxfrm() argument must be str')
    if '\0' in s:
        raise ValueError('embedded null character')
    return s

def getencoding():
    return 'UTF-8'
