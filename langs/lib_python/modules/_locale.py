# Stub: the plain C locale is what is carried here. Its text is UTF-8,
# so the C.UTF-8 spelling of the same locale is carried with it, and
# anything else is refused for the caller to skip on.
LC_CTYPE = 0
LC_NUMERIC = 1
LC_TIME = 2
LC_COLLATE = 3
LC_MONETARY = 4
LC_MESSAGES = 5
LC_ALL = 6
CHAR_MAX = 127

# The reference's locale error class, which is what callers catch.
Error = ValueError

# The names the carried locale answers to: the two plain spellings, the
# empty name that asks for the environment (which is C here), and the
# UTF-8 spelling of C, whose encoding this interpreter carries anyway.
_ANSWERED = ('C', 'POSIX', '', 'C.UTF-8')
_UTF8 = ('UTF-8', 'utf-8', 'UTF8', 'utf8', '')


def setlocale(category, locale=None):
    if category < 0 or category > 6:
        raise Error('invalid locale category')
    if locale is None:
        return 'C'
    if isinstance(locale, str):
        if locale in _ANSWERED:
            return 'C'
        raise Error('unsupported locale setting')
    if isinstance(locale, tuple) and len(locale) == 2:
        language, encoding = locale
        plain = language is None or language in ('C', 'POSIX', '')
        if plain and (encoding is None or encoding in _UTF8):
            return 'C'
    raise TypeError('Locale must be None, a string, or an iterable of two strings -- language code, encoding.')


def getlocale(category=0):
    if category == LC_ALL:
        raise TypeError('category LC_ALL is not supported')
    if category < 0 or category > 6:
        raise Error('invalid locale category')
    return (None, None)


def localeconv():
    return {'decimal_point': '.', 'thousands_sep': '', 'grouping': [], 'int_curr_symbol': '', 'currency_symbol': '', 'mon_decimal_point': '', 'mon_thousands_sep': '', 'mon_grouping': [], 'positive_sign': '', 'negative_sign': '', 'int_frac_digits': 127, 'frac_digits': 127, 'p_cs_precedes': 127, 'p_sep_by_space': 127, 'n_cs_precedes': 127, 'n_sep_by_space': 127, 'p_sign_posn': 127, 'n_sign_posn': 127}


def getencoding():
    return "UTF-8"
