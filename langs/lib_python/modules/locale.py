# Stub: the plain C locale is the only locale carried here.
LC_CTYPE = 0
LC_NUMERIC = 1
LC_TIME = 2
LC_COLLATE = 3
LC_MONETARY = 4
LC_MESSAGES = 5
LC_ALL = 6
CHAR_MAX = 127

def setlocale(category, locale=None):
    if category < 0 or category > 6:
        raise 'ValueError: invalid locale category'
    if locale is not None and locale != 'C' and locale != 'POSIX' and locale != '':
        raise 'NotImplementedError: locale.setlocale supports only C'
    return 'C'

def normalize(name):
    # The reference's own normalizing goes by a table of the codes it
    # knows, mapping each to the form it is installed under; the table
    # is not carried here. What is left of the working is the shape
    # every code is brought to: the modifier taken off, the language
    # lower and the country upper, the encoding kept as it was.
    name = name.split('@')[0]
    if '.' in name:
        head, encoding = name.split('.', 1)
    else:
        head, encoding = name, None
    parts = head.split('_')
    whole = parts[0].lower()
    if len(parts) > 1:
        whole = whole + '_' + parts[1].upper()
    if encoding is not None:
        whole = whole + '.' + encoding
    return whole

def getlocale(category=0):
    if category == LC_ALL:
        raise 'TypeError: category LC_ALL is not supported'
    if category < 0 or category > 6:
        raise 'ValueError: invalid locale category'
    return (None, None)

def localeconv():
    return {'decimal_point': '.', 'thousands_sep': '', 'grouping': [], 'int_curr_symbol': '', 'currency_symbol': '', 'mon_decimal_point': '', 'mon_thousands_sep': '', 'mon_grouping': [], 'positive_sign': '', 'negative_sign': '', 'int_frac_digits': 127, 'frac_digits': 127, 'p_cs_precedes': 127, 'p_sep_by_space': 127, 'n_cs_precedes': 127, 'n_sep_by_space': 127, 'p_sign_posn': 127, 'n_sign_posn': 127}
