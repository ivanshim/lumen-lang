# Native instruction inspection. Opcode spellings and offsets describe the
# active kernel's compiled representation, rather than CPython bytecode.

# The opcode tables below describe CPython's instruction set, which tools
# still query by number; the instructions get_instructions() yields are the
# kernel's own, named by the strings in the native hasjump set.
import opcode as _opcode_tables
from opcode import (cmp_op, opmap, opname, hasarg, hasconst, hasname, hasjrel,
                    hasjabs, hasfree, haslocal, hasexc, hascompare,
                    HAVE_ARGUMENT, EXTENDED_ARG, _specializations)
from _opcode import stack_effect

deoptmap = {specialized: base for base, family in _specializations.items()
            for specialized in family}

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
    1 << 26: 'HAS_DOCSTRING',
    1 << 27: 'METHOD',
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

# CPython's numeric tables are imported above; tools query those numbers.
# Native get_instructions() names jumps with these strings. Union them into
# the numeric sets so both a CPython opcode number and a native jump name
# match. Numbers and names cannot be sorted together, so this does not
# replace the numeric tables with sorted(hasjump).
_native_jumps = ['Skip', 'SkipCmp', 'Depart', 'Cycle', 'Choose']
hasjump = list(_opcode_tables.hasjump) + _native_jumps
hasarg = list(hasarg) + _native_jumps
hasjrel = list(hasjrel) + _native_jumps

class Positions:
    def __init__(self, values):
        self.lineno, self.col_offset, self.end_lineno, self.end_col_offset = values

class Instruction:
    def __init__(self, offset, opcode, argval, is_jump, positions):
        self.offset = offset
        self.opcode = self.opname = opcode
        self.arg = self.oparg = self.argval = argval
        self.argrepr = '' if argval is None else str(argval)
        self.positions = Positions(positions)
        self.starts_line = self.positions.lineno
        self.is_jump_target = False
        self.is_jump = is_jump
        self.cache_info = None


def get_instructions(x, *, first_line=None, show_caches=False, adaptive=False):
    """Yield real native instructions and their retained source coordinates."""
    if isinstance(x, (str, bytes)):
        x = compile(x, '<disassembly>', 'exec')
    rows = __trace_native__(2, x)
    adjustment = 0
    if first_line is not None:
        code = getattr(x, '__code__', x)
        adjustment = first_line - code.co_firstlineno
    targets = {row[2] for row in rows if row[3] and row[2] is not None}
    for offset, opcode, arg, jump, positions in rows:
        if adjustment:
            positions = (positions[0] + adjustment, positions[1] + adjustment if positions[1] is not None else None, positions[2], positions[3])
        instruction = Instruction(offset, opcode, arg, jump, positions)
        instruction.is_jump_target = offset in targets
        yield instruction

def findlinestarts(code):
    _refuse()

def findlabels(code):
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
