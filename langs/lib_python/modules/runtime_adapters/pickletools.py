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
def _buffer_genops(data, yield_end_pos=False):
    size = len(data)
    position = 0
    while position < size:
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

# Choose a generator without consulting mutable opcode descriptors before its first step.
_buffer_starts = frozenset(ord(code) for code, opcode in code2op.items()
                           if opcode.arg is None
                           or opcode.arg.reader in _fixed_readers
                           or opcode.arg.reader in _counted_readers
                           or opcode.arg.reader in _line_readers)

# Select a lazy parser without observing mutable descriptors during generator creation.
def _genops(data, yield_end_pos=False):
    if type(data) is bytes and data and data[0] in _buffer_starts:
        return _buffer_genops(data, yield_end_pos)
    return _stream_genops(data, yield_end_pos)

_stream_optimize = optimize
_memory_genops = _genops
_memory_pickler = pickle._Pickler
_memory_framer = pickle._Framer

# Assemble each complete frame once instead of calling stream methods per opcode.
def _append_frame(output, pieces, length, minimum):
    if length >= minimum:
        output.append(pickle.FRAME + length.to_bytes(8, 'little'))
    output.append(b''.join(pieces))

# Renumber referenced memo entries while keeping protocol and frame boundaries intact.
def optimize(p):
    if (type(p) is not bytes or _genops is not _memory_genops
            or pickle._Pickler is not _memory_pickler
            or pickle._Framer is not _memory_framer):
        return _stream_optimize(p)
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
