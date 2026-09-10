import os
import locale
import platform
import unicodedata
print(os.path.basename('/a/b'), os.path.dirname('/a/b'))
print(locale.setlocale(locale.LC_ALL, 'C'), locale.getlocale(), locale.localeconv()['decimal_point'])
print(platform.python_version())
print(unicodedata.lookup('LATIN SMALL LETTER E WITH ACUTE'))
print(unicodedata.normalize('NFKC', '²'))
