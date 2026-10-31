"""Opcode queries for the CPython 3.14 instruction set.

lumen's kernels do not execute CPython bytecode; this module answers the
questions CPython's `_opcode` answers about that instruction set (which
opcodes exist, which take operands, how they change the stack) from tables
of its documented properties, for tools such as `opcode` and `dis`.
"""


# Whether CPython would specialize bytecode; the native kernels never do.
ENABLE_SPECIALIZATION = False
ENABLE_SPECIALIZATION_FT = False


def _expand(*spans):
    return frozenset(n for lo, hi in spans for n in range(lo, hi))


_ARG = _expand((44, 121), (128, 129), (143, 154), (155, 196), (197, 198), (200, 201), (209,
    212), (237, 238), (239, 240), (241, 246), (247, 252), (253, 254), (255, 256), (257, 262),
    (263, 267))
_CONST = _expand((82, 83), (190, 192))
_NAME = _expand((61, 62), (64, 66), (72, 74), (80, 81), (91, 94), (96, 97), (110, 111), (115,
    117), (179, 180), (189, 190), (194, 196), (200, 201), (249, 250))
_JUMP = _expand((68, 69), (70, 71), (75, 78), (100, 104), (106, 107), (172, 177), (237, 238),
    (248, 249), (257, 261))
_FREE = _expand((62, 63), (90, 91), (97, 98), (111, 112))
_LOCAL = _expand((3, 4), (63, 64), (83, 90), (112, 115), (261, 262), (266, 267))
_EXC = _expand((263, 266))
_VALID = _expand((0, 121), (128, 212), (234, 267))
_FIXED = {0: 0, 1: -2, 2: -1, 4: -3, 5: 0, 6: 0, 7: -1, 8: -2, 9: -1, 10: -1, 11: -1, 12: 0, 13:
    -1, 14: 0, 15: 1, 16: 0, 17: 0, 18: 1, 19: 0, 20: -1, 21: 1, 22: 1, 23: 0, 24: 1, 25: 1, 26:
    1, 27: 0, 28: 0, 29: -1, 30: -1, 31: -1, 32: 1, 33: 1, 34: 1, 35: 0, 36: 0, 37: -4, 38: -3,
    39: 0, 40: 0, 41: 0, 42: 0, 43: 1, 234: -1, 235: -1, 236: -1, 238: 0, 240: 0, 246: 0, 252:
    -3, 254: 0, 256: 0, 262: 0}
_LINEAR = {44: (-1, 0), 46: (1, -1), 47: (1, -2), 48: (1, -1), 49: (1, -1), 50: (1, -1), 51: (1,
    -1), 52: (-1, -1), 53: (0, 0), 54: (-1, 0), 55: (-2, -1), 56: (-1, 0), 57: (-1, 0), 58: (0,
    0), 59: (1, 0), 60: (0, 0), 61: (-1, 0), 62: (0, 0), 63: (0, 0), 64: (0, 0), 65: (0, 0), 66:
    (-1, 0), 67: (-1, 0), 68: (-2, 0), 69: (0, 0), 70: (1, 0), 71: (0, 0), 72: (1, 0), 73: (-1,
    0), 74: (-1, 0), 75: (0, 0), 76: (0, 0), 77: (0, 0), 78: (-1, 0), 79: (-1, 0), 81: (1, 0),
    82: (1, 0), 83: (1, 0), 84: (1, 0), 85: (1, 0), 86: (1, 0), 87: (2, 0), 88: (1, 0), 89: (2,
    0), 90: (0, 0), 91: (0, 0), 93: (1, 0), 94: (1, 0), 95: (1, 0), 97: (0, 0), 98: (-2, 0), 99:
    (-2, 0), 100: (-1, 0), 101: (-1, 0), 102: (-1, 0), 103: (-1, 0), 104: (0, -1), 105: (-1, 0),
    106: (0, 0), 107: (-1, 0), 108: (-1, 0), 109: (-1, 0), 110: (-2, 0), 111: (-1, 0), 112: (-1,
    0), 113: (0, 0), 114: (-2, 0), 115: (-1, 0), 116: (-1, 0), 117: (0, 0), 119:
    (-1, 1), 120: (0, 0), 128: (0, 0), 237: (1, 0), 239: (0, 0), 241: (-1, 0), 242: (-1, 0),
    243: (-1, 0), 244: (-1, 0), 245: (0, 0), 247: (0, 0), 248: (-2, 0), 250: (-1, -1), 251: (-2,
    -1), 253: (0, 0), 255: (0, 0), 257: (0, 0), 258: (0, 0), 259: (0, 0), 260: (0, 0), 261: (1,
    0), 266: (-1, 0)}
_PARITY = {45: (-1, -1), 80: (0, 1), 92: (1, 1), 96: (-2, 1), 249: (-2, 1)}

_INTRINSIC_1 = ['INTRINSIC_1_INVALID', 'INTRINSIC_PRINT', 'INTRINSIC_IMPORT_STAR',
                'INTRINSIC_STOPITERATION_ERROR', 'INTRINSIC_ASYNC_GEN_WRAP',
                'INTRINSIC_UNARY_POSITIVE', 'INTRINSIC_LIST_TO_TUPLE', 'INTRINSIC_TYPEVAR',
                'INTRINSIC_PARAMSPEC', 'INTRINSIC_TYPEVARTUPLE', 'INTRINSIC_SUBSCRIPT_GENERIC',
                'INTRINSIC_TYPEALIAS']
_INTRINSIC_2 = ['INTRINSIC_2_INVALID', 'INTRINSIC_PREP_RERAISE_STAR', 'INTRINSIC_TYPEVAR_WITH_BOUND',
                'INTRINSIC_TYPEVAR_WITH_CONSTRAINTS', 'INTRINSIC_SET_FUNCTION_TYPE_PARAMS',
                'INTRINSIC_SET_TYPEPARAM_DEFAULT']
_NB_OPS = [('NB_ADD', '+'), ('NB_AND', '&'), ('NB_FLOOR_DIVIDE', '//'), ('NB_LSHIFT', '<<'),
           ('NB_MATRIX_MULTIPLY', '@'), ('NB_MULTIPLY', '*'), ('NB_REMAINDER', '%'), ('NB_OR', '|'),
           ('NB_POWER', '**'), ('NB_RSHIFT', '>>'), ('NB_SUBTRACT', '-'), ('NB_TRUE_DIVIDE', '/'),
           ('NB_XOR', '^'), ('NB_INPLACE_ADD', '+='), ('NB_INPLACE_AND', '&='),
           ('NB_INPLACE_FLOOR_DIVIDE', '//='), ('NB_INPLACE_LSHIFT', '<<='),
           ('NB_INPLACE_MATRIX_MULTIPLY', '@='), ('NB_INPLACE_MULTIPLY', '*='),
           ('NB_INPLACE_REMAINDER', '%='), ('NB_INPLACE_OR', '|='), ('NB_INPLACE_POWER', '**='),
           ('NB_INPLACE_RSHIFT', '>>='), ('NB_INPLACE_SUBTRACT', '-='),
           ('NB_INPLACE_TRUE_DIVIDE', '/='), ('NB_INPLACE_XOR', '^='), ('NB_SUBSCR', '[]')]

# Opcodes whose effect depends on whether the branch is taken: (taken, not taken).
_BRANCHED = {263: (2, 0), 264: (1, 0), 265: (1, 0)}

_UNPACK_EX = 118
_INT_MAX = 2 ** 31 - 1


def _as_int(value):
    n = value.__index__() if hasattr(value, '__index__') else None
    if n is None:
        raise TypeError("'%s' object cannot be interpreted as an integer" % type(value).__name__)
    if n > _INT_MAX or n < -_INT_MAX - 1:
        raise OverflowError('Python int too large to convert to C int')
    return n


def _query(table):
    def test(opcode):
        return _as_int(opcode) in table
    return test


is_valid = _query(_VALID)
has_arg = _query(_ARG)
has_const = _query(_CONST)
has_name = _query(_NAME)
has_jump = _query(_JUMP)
has_free = _query(_FREE)
has_local = _query(_LOCAL)
has_exc = _query(_EXC)


def stack_effect(opcode, oparg=None, /, *, jump=None):
    """Compute the stack effect of the opcode."""
    op = _as_int(opcode)
    arg = 0 if oparg is None else _as_int(oparg)
    if jump is not None and jump is not True and jump is not False:
        raise ValueError('stack_effect: jump must be False, True or None')
    if op in _BRANCHED:
        taken, fallthrough = _BRANCHED[op]
        return taken if jump is None else (taken if jump else fallthrough)
    if op in _FIXED:
        return _FIXED[op]
    if op == _UNPACK_EX:
        # The operand packs the counts before and after the starred target.
        return (arg & 255) + (arg >> 8)
    if op in _LINEAR:
        base, step = _LINEAR[op]
        return base + step * arg
    if op in _PARITY:
        base, step = _PARITY[op]
        return base + step * (arg & 1)
    raise ValueError('invalid opcode or oparg')


def get_intrinsic1_descs():
    return list(_INTRINSIC_1)


def get_intrinsic2_descs():
    return list(_INTRINSIC_2)


def get_special_method_names():
    return ['__enter__', '__exit__', '__aenter__', '__aexit__']


def get_nb_ops():
    return list(_NB_OPS)


def get_specialization_stats():
    # No adaptive interpreter here, so there are no statistics to report.
    return None


def get_executor(code, offset):
    # Executors belong to CPython bytecode machine, which is not here.
    raise NotImplementedError("CPython bytecode executors are unavailable")
