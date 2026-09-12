# There is no CPython bytecode here to disassemble. This runtime compiles
# Python to its own instructions, which carry none of the opnames, oparg
# numbering or stack discipline that dis reports on, so inventing an opname
# table would only mislead a caller who asked what a function compiles to.
# Every entry point says that plainly.

COMPILER_FLAG_NAMES = {
    1: 'OPTIMIZED',
    2: 'NEWLOCALS',
    4: 'VARARGS',
    8: 'VARKEYWORDS',
    16: 'NESTED',
    32: 'GENERATOR',
    64: 'NOFREE',
    128: 'COROUTINE',
    256: 'ITERABLE_COROUTINE',
    512: 'ASYNC_GENERATOR',
}

_REFUSAL = 'NotImplementedError: there is no CPython bytecode here to disassemble'

def _refuse():
    raise _REFUSAL

def code_info(x):
    _refuse()

def show_code(x, *, file=None):
    _refuse()

def dis(x=None, *, file=None, depth=None, show_caches=False, adaptive=False,
        show_offsets=False):
    _refuse()

def disassemble(co, lasti=-1, *, file=None, show_caches=False, adaptive=False):
    _refuse()

def distb(tb=None, *, file=None, show_caches=False, adaptive=False):
    _refuse()

def disco(co, lasti=-1, *, file=None):
    _refuse()

def get_instructions(x, *, first_line=None, show_caches=False, adaptive=False):
    _refuse()

def findlinestarts(code):
    _refuse()

def findlabels(code):
    _refuse()

def stack_effect(opcode, oparg=None, *, jump=None):
    _refuse()

def pretty_flags(flags):
    names = []
    left = flags
    bit = 0
    while bit < 32:
        flag = 1 << bit
        if left & flag:
            if flag in COMPILER_FLAG_NAMES:
                names.append(COMPILER_FLAG_NAMES[flag])
            else:
                names.append(hex(flag))
            left = left ^ flag
            if not left:
                return ', '.join(names)
        bit += 1
    names.append(hex(left))
    return ', '.join(names)

class Bytecode:
    def __init__(self, x, *, first_line=None, current_offset=None,
                 show_caches=False, adaptive=False):
        _refuse()
