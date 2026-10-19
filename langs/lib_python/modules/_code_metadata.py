# Code metadata over native instructions. Offsets refer to the native program,
# never to a fabricated CPython bytecode stream.
import operator
import warnings

_INTS = ('co_argcount', 'co_posonlyargcount', 'co_kwonlyargcount', 'co_nlocals',
         'co_stacksize', 'co_flags', 'co_firstlineno')
_TEXT = ('co_filename', 'co_name', 'co_qualname')
_TUPLES = ('co_consts', 'co_names', 'co_varnames', 'co_freevars', 'co_cellvars')
_BYTES = ('co_code', 'co_linetable', 'co_exceptiontable')

# Validate the public code constructor and replacement input types.
def _checked(name, value, position=None):
    if name in _INTS:
        number = operator.index(value)
        if number < -2147483648 or number > 2147483647:
            raise OverflowError('Python int too large to convert to C int')
        if position is None and number < 0:
            raise ValueError(name + ' must be a positive integer')
        return number
    wanted = str if name in _TEXT else tuple if name in _TUPLES else bytes
    if not isinstance(value, wanted):
        actual = 'None' if value is None and position is None else type(value).__name__
        prefix = "replace() argument '" + name + "'" if position is None else 'code() argument ' + str(position)
        raise TypeError(prefix + ' must be ' + wanted.__name__ + ', not ' + actual)
    if name in ('co_varnames', 'co_freevars', 'co_cellvars'):
        if any(not isinstance(item, str) for item in value):
            raise SystemError('non-string found in code slot')
    if name == 'co_names' and any(not isinstance(item, str) for item in value):
        raise SystemError('non-string found in code slot')
    return value

# Enforce layout invariants before a code object can be returned.
def _layout(argcount, posonly, kwonly, nlocals, flags, varnames):
    if argcount < 0:
        raise ValueError('code: argcount must not be negative')
    if posonly < 0:
        raise ValueError('code: posonlyargcount must not be negative')
    if kwonly < 0:
        raise ValueError('code: kwonlyargcount must not be negative')
    if nlocals < 0:
        raise ValueError('code: nlocals must not be negative')
    if flags < 0 or posonly > argcount:
        raise SystemError('Objects/codeobject.c:465: bad argument to internal function')
    if nlocals != len(varnames):
        raise ValueError('code: co_nlocals != len(co_varnames)')
    needed = argcount + kwonly + bool(flags & 4) + bool(flags & 8)
    if len(varnames) < needed:
        raise ValueError('code: co_varnames is too small')

# Validate replacement values against the complete resulting layout.
def validate_replace(code, changes):
    checked = {}
    for name, value in changes.items():
        if name not in _INTS + _TEXT + _TUPLES + _BYTES:
            raise TypeError("replace() got an unexpected keyword argument '" + name + "'")
        checked[name] = _checked(name, value)
    if any(name in checked for name in ('co_argcount', 'co_posonlyargcount', 'co_kwonlyargcount', 'co_nlocals', 'co_flags', 'co_varnames')):
        values = [checked[name] if name in checked else getattr(code, name) for name in
                  ('co_argcount', 'co_posonlyargcount', 'co_kwonlyargcount', 'co_nlocals', 'co_flags', 'co_varnames')]
        _layout(*values)
    return checked

# Rebuild native scanner images and then apply the supplied code layout.
def construct(*args):
    if len(args) < 16:
        raise TypeError('code expected at least 16 arguments, got ' + str(len(args)))
    if len(args) > 18:
        raise TypeError('code expected at most 18 arguments, got ' + str(len(args)))
    order = ('co_argcount', 'co_posonlyargcount', 'co_kwonlyargcount', 'co_nlocals',
             'co_stacksize', 'co_flags', 'co_code', 'co_consts', 'co_names',
             'co_varnames', 'co_filename', 'co_name', 'co_qualname', 'co_firstlineno',
             'co_linetable', 'co_exceptiontable', 'co_freevars', 'co_cellvars')
    values = [_checked(name, value, i + 1) for i, (name, value) in enumerate(zip(order, args))]
    _layout(values[0], values[1], values[2], values[3], values[5], values[9])
    if values[4] < 0:
        raise SystemError('Objects/codeobject.c:465: bad argument to internal function')
    values.extend([()] * (18 - len(values)))
    code = __trace_native__(12, values[6])
    return code.replace(**dict(zip(order, values)))

# Provide the unbound replacement protocol used by copy.replace.
def replace(code, /, **changes):
    return code.replace(**changes)

# Append a six-bit unsigned integer to a location table.
def _varint(number, output):
    while number >= 64:
        output.append((number & 63) | 64)
        number >>= 6
    output.append(number)

# Encode retained native source coordinates in Python's location-table format.
def location_table(code):
    output = []
    line = code.co_firstlineno
    for start, end, column, finish in code.co_positions():
        if start is None:
            output.append(248)
            continue
        output.append(240)
        delta = start - line
        _varint((-delta << 1) | 1 if delta < 0 else delta << 1, output)
        line = start
        _varint(0 if end is None else end - start, output)
        _varint(0 if column is None else column + 1, output)
        _varint(0 if finish is None else finish + 1, output)
    return bytes(output)

# Read all the standard short and long forms of a location table.
def positions(code):
    data = iter(code.co_linetable)
    line = code.co_firstlineno
    def unsigned():
        value = 0
        shift = 0
        while True:
            byte = next(data)
            value |= (byte & 63) << shift
            if not byte & 64:
                return value
            shift += 6
    def signed():
        value = unsigned()
        return -(value >> 1) if value & 1 else value >> 1
    while True:
        first = next(data, None)
        if first is None:
            return
        kind, length = (first >> 3) & 15, (first & 7) + 1
        try:
            if kind == 15:
                item = (None, None, None, None)
            elif kind == 14:
                line += signed()
                end = line + unsigned()
                column, finish = unsigned(), unsigned()
                item = (line, end, column - 1 if column else None, finish - 1 if finish else None)
            elif kind == 13:
                line += signed()
                item = (line, line, None, None)
            elif kind >= 10:
                line += kind - 10
                item = (line, line, next(data), next(data))
            else:
                second = next(data)
                column = (kind << 3) | (second >> 4)
                item = (line, line, column, column + (second & 15))
        except StopIteration:
            raise ValueError('invalid location table') from None
        item = tuple(None if part == -1 else part for part in item)
        for _ in range(length):
            yield item

# Coalesce consecutive instructions that have the same source line.
def lines(code):
    start = 0
    previous = None
    end = 0
    for position in positions(code):
        line = position[0]
        if end and line != previous:
            yield start, end, previous
            start = end
        previous = line
        end += 2
    if end:
        yield start, end, previous

# Retain the deprecated line-delta representation and its warning filters.
def lnotab(code):
    warnings.warn('co_lnotab is deprecated, use co_lines instead.', DeprecationWarning, stacklevel=2)
    output = []
    old_offset, old_line = 0, code.co_firstlineno
    for start, _, line in lines(code):
        if line is None or line == old_line:
            continue
        offset, delta = start - old_offset, line - old_line
        while offset > 255:
            output.extend((255, 0))
            offset -= 255
        while delta > 127:
            output.extend((offset, 127))
            offset = 0
            delta -= 127
        while delta < -128:
            output.extend((offset, 128))
            offset = 0
            delta += 128
        output.extend((offset, delta & 255))
        old_offset, old_line = start, line
    return bytes(output)
