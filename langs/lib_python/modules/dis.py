# Native instruction inspection. Opcode spellings and offsets describe the
# active kernel's compiled representation, rather than CPython bytecode.

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

hasjump = {'Skip', 'SkipCmp', 'Depart', 'Cycle', 'Choose'}

class Positions:
    def __init__(self, values):
        self.lineno, self.end_lineno, self.col_offset, self.end_col_offset = values

class Instruction:
    def __init__(self, offset, opcode, argval, is_jump, positions):
        self.offset = offset
        self.opcode = self.opname = opcode
        self.arg = self.argval = argval
        self.argrepr = '' if argval is None else str(argval)
        self.positions = Positions(positions)
        self.starts_line = self.positions.lineno
        self.is_jump_target = False
        self.is_jump = is_jump


def get_instructions(x, *, first_line=None, show_caches=False, adaptive=False):
    """Yield real native instructions and their retained source coordinates."""
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
