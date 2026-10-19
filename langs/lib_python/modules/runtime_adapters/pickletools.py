# Decode immutable in-memory arguments without a stream call for each field.
# Mutable inputs and nonstandard readers retain the canonical stream parser.
_stream_genops = _genops
_fixed_readers = {read_uint1: (1, False), read_uint2: (2, False),
                  read_int4: (4, True), read_uint4: (4, False),
                  read_uint8: (8, False), read_float8: (8, None)}
_counted_readers = {read_bytes1: (1, 0), read_bytes4: (4, 0),
                    read_bytes8: (8, 0), read_bytearray8: (8, 1),
                    read_unicodestring1: (1, 2),
                    read_unicodestring4: (4, 2),
                    read_unicodestring8: (8, 2),
                    read_long1: (1, 3), read_long4: (4, 4),
                    read_string1: (1, 5), read_string4: (4, 6)}
_line_readers = {read_decimalnl_short: 0, read_decimalnl_long: 1,
                 read_floatnl: 2, read_unicodestringnl: 3,
                 read_stringnl_noescape: 4, read_stringnl_noescape_pair: 5}

# Fall back at the opcode boundary so truncation and reader errors keep their wording.
def _interpreted_genops(data, yield_end_pos=False, position=0):
    size = len(data)
    while position < size:
        if not _native_code_watch[0]:
            yield from _stream_from(data, position, yield_end_pos)
            return
        start = position
        code = data[position]
        opcode = code2op.get(chr(code))
        if opcode is None:
            break
        position += 1
        arg = None
        if opcode.arg is not None:
            reader = opcode.arg.reader
            if type(reader) is not type(_stream_genops):
                break
            fixed = _fixed_readers.get(reader)
            counted = _counted_readers.get(reader)
            line = _line_readers.get(reader)
            if fixed is not None:
                width, signed = fixed
                end = position + width
                if end > size:
                    break
                if signed is None:
                    arg = _unpack('>d', data[position:end])[0]
                else:
                    arg = data[position] if width == 1 else int.from_bytes(data[position:end], 'little', signed=signed)
                position = end
            elif counted is not None:
                width, conversion = counted
                end = position + width
                if end > size:
                    break
                length = data[position] if width == 1 else int.from_bytes(data[position:end], 'little', signed=conversion in (4, 6))
                if length < 0 or length > sys.maxsize or end + length > size:
                    break
                payload = data[end:end + length]
                if conversion == 1:
                    arg = bytearray(payload)
                elif conversion == 2:
                    arg = str(payload, 'utf-8', 'surrogatepass')
                elif conversion >= 5:
                    arg = payload.decode('latin-1')
                elif conversion >= 3:
                    arg = decode_long(payload)
                else:
                    arg = payload
                position = end + length
            elif line is not None:
                end = data.find(b'\n', position)
                if end < 0:
                    break
                payload = data[position:end]
                position = end + 1
                if line == 0:
                    arg = False if payload == b'00' else True if payload == b'01' else int(payload)
                elif line == 1:
                    arg = int(payload[:-1] if payload[-1:] == b'L' else payload)
                elif line == 2:
                    arg = float(payload)
                elif line == 3:
                    arg = str(payload, 'raw-unicode-escape')
                else:
                    if b'\\' in payload:
                        break
                    if line == 5:
                        end = data.find(b'\n', position)
                        if end < 0 or b'\\' in data[position:end]:
                            break
                        arg = payload.decode('utf-8') + ' ' + data[position:end].decode('utf-8')
                        position = end + 1
                    else:
                        arg = payload.decode('utf-8')
            else:
                break
        if yield_end_pos:
            yield opcode, arg, start, position
        else:
            yield opcode, arg, start
        if code == 46:
            assert opcode.name == 'STOP'
            return
    else:
        start = position
    stream = io.BytesIO(data)
    stream.seek(start)
    yield from _stream_genops(stream, yield_end_pos)

# Describe native field layouts independently of opcode spellings.
_field_batch = __bytes_memory__
_native_mapping = code2op
_native_opcodes = tuple(code2op.get(chr(code)) for code in range(256))
_reader_type = type(_stream_genops)
_field_readers = (
    None, read_uint1, read_uint2, read_int4, read_uint4, read_uint8,
    read_float8, read_bytes1, read_bytes4, read_bytes8, read_bytearray8,
    read_unicodestring1, read_unicodestring4, read_unicodestring8,
    read_long1, read_long4, read_string1, read_string4,
    read_decimalnl_short, read_decimalnl_long, read_floatnl,
    read_unicodestringnl, read_stringnl_noescape, read_stringnl_noescape_pair)
_native_readers = {reader: tag for tag, reader in enumerate(_field_readers)}

# Identify a field only while its reader remains a recognized function.
def _field_layout(opcode):
    if opcode is None:
        return -1
    descriptor = opcode.arg
    if descriptor is None:
        return 0
    reader = descriptor.reader
    return _native_readers.get(reader, -1) if type(reader) is _reader_type else -1

_stop_byte = pickle.STOP[0]
_native_layouts = tuple(_field_layout(code2op.get(chr(code)))
                        + (256 if code == _stop_byte else 0) for code in range(256))
_native_expected = tuple(None if layout < 0 else _field_readers[layout & 255]
                         for layout in _native_layouts)
_opcode_characters = tuple(chr(code) for code in range(256))
_frame_functions = (pickle._Pickler.__init__, pickle._Framer.__init__,
                    pickle._Pickler.put, pickle._Pickler.get)
_watched_readers = tuple(dict.fromkeys((*_fixed_readers, *_counted_readers,
                                      *_line_readers, decode_long, *_frame_functions)))
_watched_bodies = tuple(reader.__code__ for reader in _watched_readers)
_native_code_watch = _field_batch(_watched_readers, _watched_bodies, '__code__', False, 6)


# Resume stream parsing at the same opcode boundary when callers replace the table.
def _stream_from(data, position, yield_end_pos):
    stream = io.BytesIO(data)
    stream.seek(position)
    return _stream_genops(stream, yield_end_pos)

# Validate each descriptor at its yield boundary, allowing changes between yields.
def _buffer_genops(data, yield_end_pos=False):
    position = 0
    while True:
        if not _native_code_watch[0] or code2op is not _native_mapping:
            yield from _stream_from(data, position, yield_end_pos)
            return
        records = _field_batch(data, position, _native_layouts, False, 3)
        if not records:
            break
        for code, argument, start, end in records:
            if not _native_code_watch[0] or code2op is not _native_mapping:
                yield from _stream_from(data, position, yield_end_pos)
                return
            opcode = code2op.get(_opcode_characters[code])
            if opcode is None:
                unchanged = False
            else:
                descriptor = opcode.arg
                expected = _native_expected[code]
                unchanged = (descriptor is None if expected is None else
                             descriptor is not None and descriptor.reader is expected)
            if not unchanged:
                yield from _interpreted_genops(data, yield_end_pos, position)
                return
            position = end
            if yield_end_pos:
                yield opcode, argument, start, end
            else:
                yield opcode, argument, start
            if code == 46:
                assert opcode.name == 'STOP'
                return
    yield from _interpreted_genops(data, yield_end_pos, position)

# Choose a generator without consulting mutable opcode descriptors before its first step.
_buffer_starts = frozenset(ord(code) for code, opcode in code2op.items()
                           if opcode.arg is None
                           or opcode.arg.reader in _fixed_readers
                           or opcode.arg.reader in _counted_readers
                           or opcode.arg.reader in _line_readers)

# Select a lazy parser without observing mutable descriptors during generator creation.
def _genops(data, yield_end_pos=False):
    if not _native_code_watch[0] or code2op is not _native_mapping:
        return _stream_genops(data, yield_end_pos)
    if type(data) is bytes and data and data[0] in _buffer_starts:
        return _buffer_genops(data, yield_end_pos)
    return _stream_genops(data, yield_end_pos)

_stream_optimize = optimize
_memory_genops = _genops
_memory_pickler = pickle._Pickler
_memory_framer = pickle._Framer

# Parse a synchronous optimization pass without per-yield descriptor lookups.
# Native fields cannot invoke Python callbacks. Changed descriptors and fields
# needing the canonical readers restart through the ordinary lazy parser.
_native_descriptors = tuple(None if op is None else op.arg for op in _native_opcodes)
_native_names = tuple(None if op is None else op.name for op in _native_opcodes)
_native_protocols = tuple(None if op is None else op.proto for op in _native_opcodes)
_plan_actions = tuple(1 if code in (112, 113, 114, 148) else
                      2 if code in (103, 104, 106) else
                      3 if code == 149 else 4 if code == 128 else 0
                      for code in range(256))

def _optimization_plan(data, compact_frame=False):
    if not _native_code_watch[0]:
        return None
    assigned = set()
    referenced = {}
    operations = []
    protocol = 0
    merge_spans = True
    span_start = span_end = -1
    header = b''
    position = 0
    checked = [False] * 256
    while True:
        records = _field_batch(data, position, _native_layouts, False, 3)
        if not records:
            return None
        for code, argument, start, end in records:
            if not checked[code]:
                opcode = code2op.get(_opcode_characters[code])
                if opcode is not _native_opcodes[code]:
                    return None
                descriptor = opcode.arg
                name, version = opcode.name, opcode.proto
                if (type(name) is not str or type(version) is not int
                        or descriptor is not _native_descriptors[code]
                        or descriptor is not None and descriptor.reader is not _native_expected[code]
                        or name != _native_names[code]
                        or version != _native_protocols[code]):
                    return None
                if not _native_code_watch[0]:
                    return None
                checked[code] = True
            position = end
            action = _plan_actions[code]
            if action and span_start >= 0:
                operations.append((span_start, span_end))
                span_start = span_end = -1
            if action == 0:
                # Hold an adjacent run locally instead of rewriting its list entry.
                if merge_spans:
                    if span_end != start:
                        if span_start >= 0:
                            operations.append((span_start, span_end))
                        span_start = start
                    span_end = end
                else:
                    operations.append((start, end))
            elif action == 1:
                if code == 148:
                    argument = len(assigned)
                assigned.add(argument)
                operations.append((-1, argument))
            elif action == 2:
                protocol = max(protocol, _native_protocols[code])
                referenced[argument] = None
                operations.append((-2, argument))
            elif action == 3:
                continue
            else:
                if start and argument >= 4:
                    return None
                protocol = max(protocol, argument)
                merge_spans = protocol < 4 or compact_frame
                if start == 0:
                    header = data[start:end]
                else:
                    operations.append((start, end))
            if code == 46:
                if span_start >= 0:
                    operations.append((span_start, span_end))
                return referenced, operations, protocol, header

# Assemble each complete frame once instead of calling stream methods per opcode.
def _append_frame(output, pieces, length, minimum):
    if length >= minimum:
        output.append(pickle.FRAME + length.to_bytes(8, 'little'))
    output.append(b''.join(pieces))

# Renumber referenced memo entries while keeping protocol and frame boundaries intact.
def optimize(p):
    if (type(p) is not bytes or _genops is not _memory_genops
            or code2op is not _native_mapping
            or pickle._Pickler is not _memory_pickler
            or pickle._Framer is not _memory_framer
            or sys.gettrace() is not None):
        return _stream_optimize(p)
    # In binary protocols a memo reference needs at most five output bytes.
    # This bound proves the entire output fits before coalescing framed spans.
    frame_limit = _memory_framer._FRAME_SIZE_TARGET
    compact_frame = (type(frame_limit) is int and len(p) * 5 <= frame_limit
                     and pickle._Pickler.__init__ is _frame_functions[0]
                     and pickle._Framer.__init__ is _frame_functions[1]
                     and pickle._Pickler.put is _frame_functions[2]
                     and pickle._Pickler.get is _frame_functions[3])
    plan = _optimization_plan(p, compact_frame)
    if plan is not None:
        referenced, operations, protocol, header = plan
    else:
        assigned = set()
        referenced = {}
        operations = []
        protocol = 0
        header = b''
        for opcode, argument, start, end in _genops(p, yield_end_pos=True):
            name = opcode.name
            if 'PUT' in name:
                assigned.add(argument)
                operations.append((-1, argument))
            elif name == 'MEMOIZE':
                index = len(assigned)
                assigned.add(index)
                operations.append((-1, index))
            elif 'FRAME' in name:
                continue
            elif 'GET' in name:
                protocol = max(protocol, opcode.proto)
                referenced[argument] = None
                operations.append((-2, argument))
            elif name == 'PROTO':
                protocol = max(protocol, argument)
                if start == 0:
                    header = p[start:end]
                else:
                    operations.append((start, end))
            else:
                operations.append((start, end))
        del assigned
    pickler = pickle._Pickler(io.BytesIO(), protocol)
    target = pickler.framer._FRAME_SIZE_TARGET
    minimum = pickler.framer._FRAME_SIZE_MIN
    if compact_frame and target != frame_limit:
        return _stream_optimize(p)
    framed = protocol >= 4
    output = [header]
    pieces = []
    length = 0
    next_index = 0
    for position, argument in operations:
        frameless = False
        if position == -1:
            if argument not in referenced:
                continue
            data = pickler.put(next_index)
            referenced[argument] = next_index
            next_index += 1
        elif position == -2:
            data = pickler.get(referenced[argument])
        else:
            data = p[position:argument]
            frameless = len(data) > target
        if framed:
            if length >= target or frameless:
                _append_frame(output, pieces, length, minimum)
                pieces = []
                length = 0
            if not frameless:
                pieces.append(data)
                length += len(data)
                continue
        output.append(data)
    if length:
        _append_frame(output, pieces, length, minimum)
    return b''.join(output)

optimize.__doc__ = _stream_optimize.__doc__
