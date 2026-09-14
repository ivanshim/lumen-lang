"""
Correct syntax for variable annotation that should fail at runtime
in a certain manner. More examples are in test_grammar and test_parser.
"""

def f_bad_ann():
    __annotations__[1] = 2

class Bad:
    bad: int

def g_bad_ann():
    __annotations__['not_here'] = 1

class D_bad_ann:
    def __init__(self, x):
        __annotations__['not_here'] = x
