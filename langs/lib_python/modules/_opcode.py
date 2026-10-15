# CPython v3.14.8 opcode catalogue; native kernels do not run these bytecodes.
ENABLE_SPECIALIZATION = False
ENABLE_SPECIALIZATION_FT = False
_has_arg = (44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 128, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 197, 200, 209, 210, 211, 237, 239, 241, 242, 243, 244, 245, 247, 248, 249, 250, 251, 253, 255)

# Check catalogue membership without interpreting bytecode.
def has_arg(opcode):
    return opcode in _has_arg

_has_const = (82, 190, 191)

# Check catalogue membership without interpreting bytecode.
def has_const(opcode):
    return opcode in _has_const

_has_name = (61, 64, 65, 72, 73, 80, 91, 92, 93, 96, 110, 115, 116, 179, 189, 194, 195, 200, 249)

# Check catalogue membership without interpreting bytecode.
def has_name(opcode):
    return opcode in _has_name

_has_jump = (68, 70, 75, 76, 77, 100, 101, 102, 103, 106, 172, 173, 174, 175, 176, 237, 248)

# Check catalogue membership without interpreting bytecode.
def has_jump(opcode):
    return opcode in _has_jump

_has_free = (62, 90, 97, 111)

# Check catalogue membership without interpreting bytecode.
def has_free(opcode):
    return opcode in _has_free

_has_local = (3, 63, 83, 84, 85, 86, 87, 88, 89, 112, 113, 114)

# Check catalogue membership without interpreting bytecode.
def has_local(opcode):
    return opcode in _has_local

_has_exc = ()

# Check catalogue membership without interpreting bytecode.
def has_exc(opcode):
    return opcode in _has_exc

_is_valid = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255)

# Check catalogue membership without interpreting bytecode.
def is_valid(opcode):
    return opcode in _is_valid


# Return the release opcode metadata.
def get_intrinsic1_descs():
    return ['INTRINSIC_1_INVALID', 'INTRINSIC_PRINT', 'INTRINSIC_IMPORT_STAR', 'INTRINSIC_STOPITERATION_ERROR', 'INTRINSIC_ASYNC_GEN_WRAP', 'INTRINSIC_UNARY_POSITIVE', 'INTRINSIC_LIST_TO_TUPLE', 'INTRINSIC_TYPEVAR', 'INTRINSIC_PARAMSPEC', 'INTRINSIC_TYPEVARTUPLE', 'INTRINSIC_SUBSCRIPT_GENERIC', 'INTRINSIC_TYPEALIAS']


# Return the release opcode metadata.
def get_intrinsic2_descs():
    return ['INTRINSIC_2_INVALID', 'INTRINSIC_PREP_RERAISE_STAR', 'INTRINSIC_TYPEVAR_WITH_BOUND', 'INTRINSIC_TYPEVAR_WITH_CONSTRAINTS', 'INTRINSIC_SET_FUNCTION_TYPE_PARAMS', 'INTRINSIC_SET_TYPEPARAM_DEFAULT']


# Return the release opcode metadata.
def get_special_method_names():
    return ['__enter__', '__exit__', '__aenter__', '__aexit__']


# Return the release opcode metadata.
def get_nb_ops():
    return [('NB_ADD', '+'), ('NB_AND', '&'), ('NB_FLOOR_DIVIDE', '//'), ('NB_LSHIFT', '<<'), ('NB_MATRIX_MULTIPLY', '@'), ('NB_MULTIPLY', '*'), ('NB_REMAINDER', '%'), ('NB_OR', '|'), ('NB_POWER', '**'), ('NB_RSHIFT', '>>'), ('NB_SUBTRACT', '-'), ('NB_TRUE_DIVIDE', '/'), ('NB_XOR', '^'), ('NB_INPLACE_ADD', '+='), ('NB_INPLACE_AND', '&='), ('NB_INPLACE_FLOOR_DIVIDE', '//='), ('NB_INPLACE_LSHIFT', '<<='), ('NB_INPLACE_MATRIX_MULTIPLY', '@='), ('NB_INPLACE_MULTIPLY', '*='), ('NB_INPLACE_REMAINDER', '%='), ('NB_INPLACE_OR', '|='), ('NB_INPLACE_POWER', '**='), ('NB_INPLACE_RSHIFT', '>>='), ('NB_INPLACE_SUBTRACT', '-='), ('NB_INPLACE_TRUE_DIVIDE', '/='), ('NB_INPLACE_XOR', '^='), ('NB_SUBSCR', '[]')]


# Refuse operations that require the CPython bytecode machine.
def stack_effect(opcode, oparg=None, *, jump=None):
    raise NotImplementedError("CPython bytecode stack effects are unavailable")

# Refuse executor inspection on the native kernels.
def get_executor(code, offset):
    raise NotImplementedError("CPython bytecode executors are unavailable")
