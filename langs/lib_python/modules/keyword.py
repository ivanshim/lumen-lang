# From CPython 3.14, Lib/keyword.py, the word lists and the two questions,
# written out here so difflib's doctests can ask them.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# The PSF license is kept in tests/python/LICENSE.
kwlist = [
    'False',
    'None',
    'True',
    'and',
    'as',
    'assert',
    'async',
    'await',
    'break',
    'class',
    'continue',
    'def',
    'del',
    'elif',
    'else',
    'except',
    'finally',
    'for',
    'from',
    'global',
    'if',
    'import',
    'in',
    'is',
    'lambda',
    'nonlocal',
    'not',
    'or',
    'pass',
    'raise',
    'return',
    'try',
    'while',
    'with',
    'yield',
]

softkwlist = [
    '_',
    'case',
    'lazy',
    'match',
    'type',
]

def iskeyword(word):
    return word in kwlist

def issoftkeyword(word):
    return word in softkwlist
