# The host C library owns the process locale. Categories match glibc.

class Error(Exception):
    pass

LC_CTYPE = 0
LC_NUMERIC = 1
LC_TIME = 2
LC_COLLATE = 3
LC_MONETARY = 4
LC_MESSAGES = 5
LC_ALL = 6
# Grouping sentinels in the pure-Python formatter still use 127.
CHAR_MAX = 127

def _known_category(category):
    return isinstance(category, int) and 0 <= category <= LC_ALL

def setlocale(category, locale=None):
    if not _known_category(category):
        raise Error('invalid locale category')
    if locale is None:
        found = __clock('locale', category)
    else:
        if not isinstance(locale, str):
            raise TypeError('locale name must be a string')
        found = __clock('locale', category, locale)
    if found is None:
        raise Error('unsupported locale setting')
    return found

def getlocale(category=LC_CTYPE):
    if category == LC_ALL:
        raise TypeError('category LC_ALL is not supported')
    if not _known_category(category):
        raise Error('invalid locale category')
    name = setlocale(category)
    if name in ('C', 'POSIX'):
        return (None, None)
    dot = name.find('.')
    if dot < 0:
        return (name, None)
    return (name[:dot], name[dot + 1:])

_CONV_NAMES = (
    'decimal_point', 'thousands_sep', 'grouping',
    'int_curr_symbol', 'currency_symbol',
    'mon_decimal_point', 'mon_thousands_sep', 'mon_grouping',
    'positive_sign', 'negative_sign',
    'int_frac_digits', 'frac_digits',
    'p_cs_precedes', 'p_sep_by_space',
    'n_cs_precedes', 'n_sep_by_space',
    'p_sign_posn', 'n_sign_posn',
)

def localeconv():
    raw = __clock('conv')
    out = {}
    index = 0
    for name in _CONV_NAMES:
        value = raw[index]
        if name == 'grouping' or name == 'mon_grouping':
            value = list(value)
        out[name] = value
        index += 1
    return out

def strcoll(a, b):
    if not isinstance(a, str):
        raise TypeError('strcoll() argument 1 must be str')
    if not isinstance(b, str):
        raise TypeError('strcoll() argument 2 must be str')
    if '\0' in a or '\0' in b:
        raise ValueError('embedded null character')
    return __clock('order', a, b)

def strxfrm(s):
    if not isinstance(s, str):
        raise TypeError('strxfrm() argument must be str')
    if '\0' in s:
        raise ValueError('embedded null character')
    return __clock('fold', s)

def getencoding():
    return 'UTF-8'
