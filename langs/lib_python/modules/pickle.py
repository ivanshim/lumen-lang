# Pickle reductions use the Python reconstruction protocol. The byte
# envelope remains the runtime's private marshal-based representation,
# except for boolean roots using standard pickle encodings for protocols 0--5.
import marshal
import sys

HIGHEST_PROTOCOL = 5
DEFAULT_PROTOCOL = 5


class PickleError(Exception):
    pass


class PicklingError(PickleError):
    pass


class UnpicklingError(PickleError):
    pass


def _reach(protocol):
    if protocol is None:
        return DEFAULT_PROTOCOL
    if protocol < 0:
        return HIGHEST_PROTOCOL
    if protocol > HIGHEST_PROTOCOL:
        raise ValueError('pickle protocol must be <= ' + str(HIGHEST_PROTOCOL))
    return protocol


def _new_object(cls, args=()):
    return cls.__new__(cls, *args)


def _new_builtin(cls, value):
    return __rebuild_native__(('instance', cls, value))


def _global(module, name):
    if module == '__main__':
        parts = name.split('.')
        import __main__
        try:
            owner = getattr(__main__, parts[0])
        except AttributeError:
            owner = __program_namespace()[parts[0]]
        for part in parts[1:]:
            owner = getattr(owner, part)
        return owner
    if module == '__builtin__':
        module = 'builtins'
    if module == 'builtins' and name == 'NotImplemented':
        return NotImplemented
    if module == 'builtins' and name == 'Ellipsis':
        return Ellipsis
    if module == 'builtins' and name == 'xrange':
        name = 'range'
    if module == 'pickle' and name == '_new_object':
        return _new_object
    if module == 'pickle' and name == '_new_builtin':
        return _new_builtin
    owner = __import__(module)
    if module in sys.modules:
        owner = sys.modules[module]
    for part in name.split('.'):
        owner = getattr(owner, part)
    return owner


def _global_name(value):
    if value is NotImplemented:
        return ('builtins', 'NotImplemented')
    if value is Ellipsis:
        return ('builtins', 'Ellipsis')
    if value is bytearray:
        return ('builtins', 'bytearray')
    if value is _new_object:
        return ('pickle', '_new_object')
    if value is _new_builtin:
        return ('pickle', '_new_builtin')
    if not isinstance(value, type) and type(value) not in (type(_global_name), type(iter)):
        return None
    for candidate, name in ((iter, 'iter'), (range, 'range'), (reversed, 'reversed'), (enumerate, 'enumerate'), (map, 'map'), (filter, 'filter'), (zip, 'zip'), (abs, 'abs'), (list, 'list'), (dict, 'dict'), (set, 'set'), (frozenset, 'frozenset')):
        if candidate is value:
            return ('builtins', name)
    import builtins
    for name in dir(builtins):
        if name.startswith('#') or name.startswith('\0'):
            continue
        candidate = getattr(builtins, name)
        if candidate is value:
            return ('builtins', name)
    name = getattr(value, '__qualname__', getattr(value, '__name__', None))
    module = getattr(value, '__module__', None)
    if name is not None and '<locals>' not in name:
        if module is not None:
            try:
                advertised = _global(module, name)
            except (ImportError, AttributeError, KeyError):
                advertised = None
            if advertised is value:
                return (module, name)
        parts = name.split('.')
        # The module the value names as its own first, then the rest:
        # the reference finds the value under its own name where it
        # lives, before anything a caller happened to import.
        module_names = ([module] if module is not None else []) + [m for m in list(sys.modules) if m != module]
        for module_name in module_names:
            owner = sys.modules.get(module_name)
            if owner is None:
                continue
            if parts[0] not in dir(owner):
                continue
            candidate = owner
            for part in parts:
                candidate = getattr(candidate, part, None)
            if candidate is value:
                return (module_name, name)
    namespace = __program_namespace()
    for binding in namespace:
        if binding.startswith('#') or binding.startswith('\0'):
            continue
        candidate = namespace[binding]
        if candidate is value:
            # Instances are global only when their reduction says so.
            if isinstance(value, type) or callable(value):
                try:
                    published = _global('__main__', binding)
                except (AttributeError, KeyError):
                    continue
                if published is value:
                    return ('__main__', binding)
    if name is not None and module is not None and isinstance(value, type):
        return (module, name)
    return None


def _reduction_hook(value, name):
    # The final MRO entry is the root object, whose native hooks have a
    # separate fallback. Keep hooks declared by every more specific class.
    lineage = getattr(type(value), '__mro__', None)
    if lineage is None:
        return getattr(value, name, None)
    for cls in lineage[:-1]:
        if name in getattr(cls, '__dict__', {}):
            return getattr(value, name, None)
    return None


def _state(value):
    method = _reduction_hook(value, '__getstate__')
    if method is not None:
        return method()
    namespace = __reduce_native__(value, True)
    slots = {}
    for cls in type(value).__mro__:
        names = getattr(cls, '__slots__', ())
        if isinstance(names, str):
            names = (names,)
        for name in names:
            if name not in ('__dict__', '__weakref__'):
                if name.startswith('__') and not name.endswith('__'):
                    name = '_' + cls.__name__.lstrip('_') + name
                if hasattr(value, name):
                    slots[name] = getattr(value, name)
    if slots:
        return (namespace, slots)
    return namespace


def _reduce(value, protocol):
    if type(value) is slice:
        return (slice, (value.start, value.stop, value.step))
    if type(value) is bytearray:
        return (bytearray, (bytes(value),))
    from copyreg import dispatch_table
    reductor = dispatch_table.get(type(value))
    if reductor is not None:
        return reductor(value)
    forwarded = __reduce_native__(value)
    if forwarded is not None and forwarded[0] == 'handed':
        return _reduce(forwarded[1], protocol)
    method = _reduction_hook(value, '__reduce_ex__')
    if method is not None:
        return method(protocol)
    method = _reduction_hook(value, '__reduce__')
    if method is not None:
        return method()
    native = __reduce_native__(value)
    if native is not None and native[0] == 'numbered' and native[1] is not None:
        return (native[1], (native[2], native[3]), _state(value))
    cls = type(value)
    if isinstance(value, BaseException):
        return value.__reduce__()
    for base in (list, dict, set, frozenset, tuple, int, float, str):
        if isinstance(value, base) and cls is not base:
            return (_new_builtin, (cls, __reduce_native__(value, False)), _state(value))
    slots = getattr(cls, '__slots__', ())
    if protocol < 2 and slots and _reduction_hook(value, '__getstate__') is None:
        raise TypeError('a class that defines __slots__ without defining __getstate__ cannot be pickled')
    args = ()
    method = getattr(value, '__getnewargs__', None)
    if protocol >= 2 and method is not None:
        args = method()
    return (_new_object, (cls, args), _state(value))


def _apply_state(value, state):
    if state is None:
        return
    method = getattr(value, '__setstate__', None)
    if method is not None:
        method(state)
        return
    slots = None
    if isinstance(state, tuple) and len(state) == 2:
        state, slots = state
    if state is not None:
        for name, item in state.items():
            setattr(value, name, item)
    if slots is not None:
        for name, item in slots.items():
            setattr(value, name, item)


class _Writer(marshal._Writer):
    def __init__(self, protocol):
        super().__init__(True, self.refused)
        self.protocol = protocol
        self.keepalive = []

    def held(self, value):
        self.keepalive.append(value)
        return super().held(value)

    def refused(self, value):
        raise PicklingError('cannot pickle ' + repr(value))

    def put(self, value):
        cls = type(value)
        if value is None or cls in (bool, int, float, complex, str, bytes, tuple, list, dict, set, frozenset):
            super().put(value)
            return
        if cls is slice:
            self.pieces.append('Q')
            self.put(value.start)
            self.put(value.stop)
            self.put(value.step)
            return
        global_name = _global_name(value)
        if global_name is not None:
            self.pieces.append('G')
            self.put(global_name[0])
            self.put(global_name[1])
            return
        reduction = _reduce(value, self.protocol)
        if isinstance(reduction, str):
            try:
                existing = _global('__main__', reduction)
            except (KeyError, AttributeError):
                raise PicklingError('global name does not refer to the object being pickled')
            if existing is not value:
                raise PicklingError('global name does not refer to the object being pickled')
            self.pieces.append('G')
            self.put('__main__')
            self.put(reduction)
            return
        if not isinstance(reduction, tuple) or not 2 <= len(reduction) <= 6:
            raise PicklingError('__reduce__ must return a string or tuple of two to six elements')
        if not callable(reduction[0]) or not isinstance(reduction[1], tuple):
            raise PicklingError('invalid reduction callable or arguments')
        if self.held(value):
            return
        self.pieces.append('O')
        self.put(reduction[0])
        self.put(reduction[1])
        # The new object enters the reader memo before its state is read.
        self.put(reduction[2] if len(reduction) > 2 else None)
        self.put(list(reduction[3]) if len(reduction) > 3 and reduction[3] is not None else None)
        self.put(list(reduction[4]) if len(reduction) > 4 and reduction[4] is not None else None)
        self.put(reduction[5] if len(reduction) > 5 else None)


class _Reader(marshal._Reader):
    def get(self):
        tag = self.text[self.at:self.at + 1]
        if tag == 'G':
            self.at += 1
            module = self.get()
            name = self.get()
            return _global(module, name)
        if tag == 'O':
            self.at += 1
            where = self.reserve()
            constructor = self.get()
            args = self.get()
            value = constructor(*args)
            self.memo[where] = value
            state = self.get()
            items = self.get()
            pairs = self.get()
            setter = self.get()
            if items is not None:
                for item in items:
                    value.append(item)
            if pairs is not None:
                for key, item in pairs:
                    value[key] = item
            if setter is not None:
                setter(value, state)
            else:
                _apply_state(value, state)
            return value
        return super().get()


def dumps(obj, protocol=None, *, fix_imports=True, buffer_callback=None):
    protocol = _reach(protocol)
    if type(obj) is bool:
        if protocol < 2:
            return b'I01\n.' if obj else b'I00\n.'
        return b'\x80' + bytes([protocol]) + (b'\x88.' if obj else b'\x89.')
    writer = _Writer(protocol)
    writer.put(obj)
    return b'LP1\n' + writer.text().encode()


def loads(data, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
    if data == b'I01\n.':
        return True
    if data == b'I00\n.':
        return False
    if len(data) == 4 and data[0] == 128 and 2 <= data[1] <= HIGHEST_PROTOCOL and data[3] == 46:
        if data[2] == 136:
            return True
        if data[2] == 137:
            return False
    if isinstance(data, str):
        raise TypeError('a bytes-like object is required, not str')
    if data[:4] == b'LP1\n':
        return _Reader(data[4:].decode()).get()
    return _read_protocol(data, encoding)



# The reference keeps its pure-Python loads beside the dispatching
# one; the loads here is that pure one already.
_loads = loads

def dump(obj, file, protocol=None, *, fix_imports=True, buffer_callback=None):
    file.write(dumps(obj, protocol, fix_imports=fix_imports, buffer_callback=buffer_callback))


def load(file, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
    return loads(file.read(), fix_imports=fix_imports, encoding=encoding, errors=errors, buffers=buffers)


class Pickler:
    def __init__(self, file, protocol=None, *, fix_imports=True, buffer_callback=None):
        self.file = file
        self.protocol = _reach(protocol)

    def dump(self, obj):
        dump(obj, self.file, self.protocol)


class Unpickler:
    def __init__(self, file, **kwargs):
        self.file = file

    def load(self):
        return load(self.file)


def _decode_binstring(data, encoding):
    # A binstring's bytes are text in the encoding the reader was given:
    # latin1 keeps each byte as its own letter; bytes hands the bytes
    # over whole; ASCII refuses a byte that is not one.
    if encoding == 'bytes':
        return bytes(data)
    if encoding == 'latin1':
        out = ''
        for byte in data:
            out += chr(byte)
        return out
    if encoding == 'utf-8':
        return bytes(data).decode('utf-8')
    out = ''
    for byte in data:
        if byte > 127:
            raise UnicodeDecodeError('ascii', bytes([byte]), 0, 1, 'ordinal not in range(128)')
        out += chr(byte)
    return out

def _unescape_string(text):
    # A protocol-0 string is written as its repr: quotes around escapes.
    out = ''
    at = 0
    while at < len(text):
        ch = text[at]
        if ch != '\\':
            out += ch
            at += 1
            continue
        at += 1
        esc = text[at]
        at += 1
        if esc == 'n':
            out += '\n'
        elif esc == 'r':
            out += '\r'
        elif esc == 't':
            out += '\t'
        elif esc == '\\':
            out += '\\'
        elif esc == "'":
            out += "'"
        elif esc == '"':
            out += '"'
        elif esc == 'x':
            out += chr(int(text[at:at + 2], 16))
            at += 2
        elif esc == 'u':
            out += chr(int(text[at:at + 4], 16))
            at += 4
        elif esc in '01234567':
            digits = esc
            while len(digits) < 3 and at < len(text) and text[at] in '01234567':
                digits += text[at]
                at += 1
            out += chr(int(digits, 8))
        else:
            out += esc
    return out

def _read_protocol(data, encoding='ASCII'):
    # The stack machine also accepts older standard range-iterator pickles.
    stack = []
    marks = []
    memo = {}
    at = 0
    while at < len(data):
        op = data[at]
        at += 1
        if op == 128:
            at += 1
        elif op == 149:
            at += 8
        elif op == 46:
            return stack.pop()
        elif op == 40:
            marks.append(len(stack))
        elif op == 99:
            end = data.index(b'\n', at)
            module = data[at:end].decode()
            at = end + 1
            end = data.index(b'\n', at)
            name = data[at:end].decode()
            at = end + 1
            stack.append(_global(module, name))
        elif op == 147:
            name = stack.pop()
            module = stack.pop()
            stack.append(_global(module, name))
        elif op in (73, 76):
            end = data.index(b'\n', at)
            number = data[at:end].decode()
            at = end + 1
            if number.endswith('L'):
                number = number[:-1]
            stack.append(int(number))
        elif op == 75:
            stack.append(data[at])
            at += 1
        elif op in (74, 77, 138, 139):
            size = 4 if op == 74 else 2
            if op == 138:
                size = data[at]
                at += 1
            elif op == 139:
                size = int.from_bytes(data[at:at + 4], 'little')
                at += 4
            stack.append(int.from_bytes(data[at:at + size], 'little', signed=op != 77))
            at += size
        elif op in (140, 88):
            size = data[at] if op == 140 else int.from_bytes(data[at:at + 4], 'little')
            at += 1 if op == 140 else 4
            stack.append(data[at:at + size].decode())
            at += size
        elif op == 116:
            mark = marks.pop()
            value = tuple(stack[mark:])
            del stack[mark:]
            stack.append(value)
        elif op in (133, 134, 135):
            size = op - 132
            value = tuple(stack[-size:])
            del stack[-size:]
            stack.append(value)
        elif op == 41:
            stack.append(())
        elif op == 82:
            args = stack.pop()
            constructor = stack.pop()
            stack.append(constructor(*args))
        elif op == 98:
            state = stack.pop()
            _apply_state(stack[-1], state)
        elif op == 148:
            memo[len(memo)] = stack[-1]
        elif op in (112, 103):
            end = data.index(b'\n', at)
            index = int(data[at:end])
            at = end + 1
            if op == 112:
                memo[index] = stack[-1]
            else:
                stack.append(memo[index])
        elif op == 78:
            stack.append(None)
        elif op in (113, 104):
            index = data[at]
            at += 1
            if op == 113:
                memo[index] = stack[-1]
            else:
                stack.append(memo[index])
        elif op == 136:
            stack.append(True)
        elif op == 137:
            stack.append(False)
        elif op == 78:
            stack.append(None)
        elif op == 83:
            end = data.index(b'\n', at)
            text = data[at:end].decode()
            at = end + 1
            stack.append(_unescape_string(text[1:len(text) - 1]))
        elif op == 86:
            end = data.index(b'\n', at)
            text = data[at:end].decode()
            at = end + 1
            stack.append(_unescape_string(text))
        elif op == 85:
            size = data[at]
            at += 1
            stack.append(_decode_binstring(data[at:at + size], encoding))
            at += size
        elif op == 84:
            size = int.from_bytes(data[at:at + 4], 'little')
            at += 4
            stack.append(_decode_binstring(data[at:at + size], encoding))
            at += size
        elif op == 70:
            end = data.index(b'\n', at)
            stack.append(float(data[at:end].decode()))
            at = end + 1
        elif op == 71:
            import struct
            stack.append(struct.unpack('>d', data[at:at + 8])[0])
            at += 8
        elif op == 93:
            stack.append([])
        elif op == 125:
            stack.append({})
        elif op == 108:
            mark = marks.pop()
            value = list(stack[mark:])
            del stack[mark:]
            stack.append(value)
        elif op == 97:
            # The value comes off first: the list beneath it appends it.
            value = stack.pop()
            stack[-1].append(value)
        elif op == 101:
            mark = marks.pop()
            target = stack[mark - 1]
            target.extend(stack[mark:])
            del stack[mark:]
        elif op == 100:
            mark = marks.pop()
            value = {}
            for pair_at in range(mark, len(stack), 2):
                value[stack[pair_at]] = stack[pair_at + 1]
            del stack[mark:]
            stack.append(value)
        elif op == 115:
            value = stack.pop()
            key = stack.pop()
            stack[-1][key] = value
        elif op == 117:
            mark = marks.pop()
            target = stack[mark - 1]
            for pair_at in range(mark, len(stack), 2):
                target[stack[pair_at]] = stack[pair_at + 1]
            del stack[mark:]
        elif op in (112, 103):
            end = data.index(b'\n', at)
            index = int(data[at:end].decode())
            at = end + 1
            if op == 112:
                memo[index] = stack[-1]
            else:
                stack.append(memo[index])
        elif op in (114, 106):
            index = int.from_bytes(data[at:at + 4], 'little')
            at += 4
            if op == 114:
                memo[index] = stack[-1]
            else:
                stack.append(memo[index])
        else:
            raise UnpicklingError('unsupported pickle opcode: ' + str(op))
    raise UnpicklingError('pickle data was truncated')
