# Write standard pickle opcodes for protocols 0 through 5;
# the reduction of each value comes from the runtime through _reduce.
import codecs
import copyreg
import marshal
import sys
from struct import pack as _pack

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


def _whichmodule(obj, name):
    parts = name.split('.')
    module = getattr(obj, '__module__', None)
    candidates = []
    if module is not None:
        candidates.append(module)
    candidates.append('__main__')
    for key in list(sys.modules):
        if key not in candidates:
            candidates.append(key)
    for module_name in candidates:
        if module_name == '__main__':
            try:
                candidate = _global('__main__', name)
            except (ImportError, AttributeError, KeyError):
                continue
        else:
            owner = sys.modules.get(module_name)
            if owner is None:
                continue
            candidate = owner
            found = True
            for part in parts:
                candidate = getattr(candidate, part, None)
                if candidate is None:
                    found = False
                    break
            if not found:
                continue
        if candidate is obj:
            return module_name
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
    if isinstance(namespace, dict) and not namespace:
        namespace = None
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


# Validate reconstruction hooks without replacing their results or errors.
def _newargs(value):
    method_ex = getattr(value, '__getnewargs_ex__', None)
    if method_ex is not None:
        result = method_ex()
        if not isinstance(result, tuple):
            raise TypeError("__getnewargs_ex__ should return a tuple, not '" + type(result).__name__ + "'")
        size = tuple.__len__(result)
        if size != 2:
            raise ValueError('__getnewargs_ex__ should return a tuple of length 2, not ' + str(size))
        args = tuple.__getitem__(result, 0)
        keywords = tuple.__getitem__(result, 1)
        if not isinstance(args, tuple):
            raise TypeError("first item of the tuple returned by __getnewargs_ex__ must be a tuple, not '" + type(args).__name__ + "'")
        if not isinstance(keywords, dict):
            raise TypeError("second item of the tuple returned by __getnewargs_ex__ must be a dict, not '" + type(keywords).__name__ + "'")
        return args, keywords, True
    method = getattr(value, '__getnewargs__', None)
    if method is None:
        return (), None, False
    args = method()
    if not isinstance(args, tuple):
        raise TypeError("__getnewargs__ should return a tuple, not '" + type(args).__name__ + "'")
    return args, None, True


def _reduce(value, protocol):
    if type(value) is slice:
        return (slice, (value.start, value.stop, value.step))
    if type(value) is bytearray:
        return (bytearray, (bytes(value),))
    if type(value) is complex:
        return (complex, (value.real, value.imag))
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
    for base in (list, dict, set, frozenset, tuple, int, float, str, bytes, bytearray, complex):
        if isinstance(value, base) and cls is not base:
            state = _state(value)
            if base is set or base is frozenset:
                return (cls, (list(value),), state)
            if base is bytearray:
                if protocol < 3:
                    return (cls, (str(bytes(value), 'latin-1'), 'latin-1'), state)
                return (cls, (bytes(value),), state)
            if base is complex:
                return (copyreg.__newobj__, (cls, value.real, value.imag), state)
            args, keywords, supplied = _newargs(value)
            if keywords:
                if base is list:
                    return (copyreg.__newobj_ex__, (cls, args, keywords), state, iter(value))
                if base is dict:
                    return (copyreg.__newobj_ex__, (cls, args, keywords), state, None, iter(value.items()))
                return (copyreg.__newobj_ex__, (cls, args, keywords), state)
            if base is list:
                return (copyreg.__newobj__, (cls,) + args, state, iter(value))
            if base is dict:
                return (copyreg.__newobj__, (cls,) + args, state, None, iter(value.items()))
            if args or supplied:
                return (copyreg.__newobj__, (cls,) + args, state)
            return (copyreg.__newobj__, (cls, base(value)), state)
    slots = getattr(cls, '__slots__', ())
    if protocol < 2 and slots and _reduction_hook(value, '__getstate__') is None:
        raise TypeError('a class that defines __slots__ without defining __getstate__ cannot be pickled')
    state = _state(value)
    if protocol < 2:
        return (copyreg._reconstructor, (cls, object, None), state)
    args, keywords, _supplied = _newargs(value)
    if keywords:
        return (copyreg.__newobj_ex__, (cls, args, keywords), state)
    return (copyreg.__newobj__, (cls,) + args, state)


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


# The standard pickle opcodes, exactly as CPython's Lib/pickle.py spells
# them, so that the stream is protocol-sensitive and readable by the
# reference unpickler.
MARK = b'('
STOP = b'.'
POP = b'0'
POP_MARK = b'1'
DUP = b'2'
FLOAT = b'F'
INT = b'I'
BININT = b'J'
BININT1 = b'K'
LONG = b'L'
BININT2 = b'M'
NONE = b'N'
REDUCE = b'R'
STRING = b'S'
BINSTRING = b'T'
SHORT_BINSTRING = b'U'
UNICODE = b'V'
BINUNICODE = b'X'
APPEND = b'a'
BUILD = b'b'
GLOBAL = b'c'
DICT = b'd'
EMPTY_DICT = b'}'
APPENDS = b'e'
GET = b'g'
BINGET = b'h'
LONG_BINGET = b'j'
LIST = b'l'
EMPTY_LIST = b']'
PUT = b'p'
BINPUT = b'q'
LONG_BINPUT = b'r'
SETITEM = b's'
TUPLE = b't'
EMPTY_TUPLE = b')'
SETITEMS = b'u'
BINFLOAT = b'G'
TRUE = b'I01\n'
FALSE = b'I00\n'
PROTO = b'\x80'
NEWOBJ = b'\x81'
TUPLE1 = b'\x85'
TUPLE2 = b'\x86'
TUPLE3 = b'\x87'
NEWTRUE = b'\x88'
NEWFALSE = b'\x89'
LONG1 = b'\x8a'
LONG4 = b'\x8b'
BINBYTES = b'B'
SHORT_BINBYTES = b'C'
SHORT_BINUNICODE = b'\x8c'
BINUNICODE8 = b'\x8d'
BINBYTES8 = b'\x8e'
EMPTY_SET = b'\x8f'
ADDITEMS = b'\x90'
FROZENSET = b'\x91'
NEWOBJ_EX = b'\x92'
STACK_GLOBAL = b'\x93'
MEMOIZE = b'\x94'
FRAME = b'\x95'
BYTEARRAY8 = b'\x96'

_tuplesize2code = [EMPTY_TUPLE, TUPLE1, TUPLE2, TUPLE3]


def _encode_long(x):
    if x == 0:
        return b''
    nbytes = (x.bit_length() >> 3) + 1
    result = x.to_bytes(nbytes, byteorder='little', signed=True)
    if x < 0 and nbytes > 1:
        if result[-1] == 0xff and (result[-2] & 0x80) != 0:
            result = result[:-1]
    return result


def _batches(items, size):
    import itertools
    return itertools.batched(items, size)


class _Pickler:
    def __init__(self, protocol=None):
        self.proto = _reach(protocol)
        self.bin = self.proto >= 1
        self.memo = {}
        self.buffer = bytearray()
        self.frame = None

    def _key(self, obj):
        try:
            return id(obj)
        except Exception:
            name = _global_name(obj)
            return ('name',) + (name if name is not None else (repr(obj),))

    def write(self, data):
        if self.frame is None:
            self.buffer += data
        else:
            self.frame += data
            if len(self.frame) >= 65536:
                self.commit_frame()

    def commit_frame(self):
        if self.frame:
            data = bytes(self.frame)
            if len(data) >= 4:
                self.buffer += FRAME + _pack('<Q', len(data))
            self.buffer += data
        self.frame = bytearray()

    def dump(self, obj):
        if self.proto >= 2:
            self.write(PROTO + _pack('<B', self.proto))
        if self.proto >= 4:
            self.frame = bytearray()
        self.save(obj)
        self.write(STOP)
        if self.frame is not None:
            self.commit_frame()
            self.frame = None
        return bytes(self.buffer)

    def memoize(self, obj):
        idx = len(self.memo)
        self.write(self.put(idx))
        self.memo[self._key(obj)] = (idx, obj)

    def put(self, idx):
        if self.proto >= 4:
            return MEMOIZE
        if self.bin:
            if idx < 256:
                return BINPUT + _pack('<B', idx)
            return LONG_BINPUT + _pack('<I', idx)
        return PUT + repr(idx).encode('ascii') + b'\n'

    def get(self, idx):
        if self.bin:
            if idx < 256:
                return BINGET + _pack('<B', idx)
            return LONG_BINGET + _pack('<I', idx)
        return GET + repr(idx).encode('ascii') + b'\n'

    def save(self, obj):
        if self._key(obj) in self.memo:
            self.write(self.get(self.memo[self._key(obj)][0]))
            return
        cls = type(obj)
        if obj is None:
            self.write(NONE)
        elif cls is bool:
            if self.proto >= 2:
                self.write(NEWTRUE if obj else NEWFALSE)
            else:
                self.write(TRUE if obj else FALSE)
        elif cls is int:
            self.save_long(obj)
        elif cls is float:
            self.save_float(obj)
        elif cls is str:
            self.save_str(obj)
        elif cls is bytes:
            self.save_bytes(obj)
        elif cls is bytearray:
            self.save_bytearray(obj)
        elif cls is tuple:
            self.save_tuple(obj)
        elif cls is list:
            self.save_list(obj)
        elif cls is dict:
            self.save_dict(obj)
        elif cls is set:
            self.save_set(obj)
        elif cls is frozenset:
            self.save_frozenset(obj)
        elif isinstance(obj, type):
            self.save_type(obj)
        else:
            if _global_name(obj) is not None:
                self.save_global(obj)
                return
            reduction = _reduce(obj, self.proto)
            if isinstance(reduction, str):
                self.save_global(obj, reduction)
                return
            if not isinstance(reduction, tuple) or not 2 <= len(reduction) <= 6:
                raise PicklingError('__reduce__ must return a string or tuple of two to six elements')
            reduction = reduction + (None,) * (6 - len(reduction))
            self.save_reduce(reduction[0], reduction[1], reduction[2], reduction[3], reduction[4], reduction[5], obj=obj)

    def save_long(self, obj):
        if self.bin:
            if obj >= 0:
                if obj <= 0xff:
                    self.write(BININT1 + _pack('<B', obj))
                    return
                if obj <= 0xffff:
                    self.write(BININT2 + _pack('<H', obj))
                    return
            if -0x80000000 <= obj <= 0x7fffffff:
                self.write(BININT + _pack('<i', obj))
                return
        if self.proto >= 2:
            encoded = _encode_long(obj)
            size = len(encoded)
            if size < 256:
                self.write(LONG1 + _pack('<B', size) + encoded)
            else:
                self.write(LONG4 + _pack('<i', size) + encoded)
            return
        if -0x80000000 <= obj <= 0x7fffffff:
            self.write(INT + repr(obj).encode('ascii') + b'\n')
        else:
            self.write(LONG + repr(obj).encode('ascii') + b'L\n')

    def save_float(self, obj):
        if self.bin:
            self.write(BINFLOAT + _pack('>d', obj))
        else:
            self.write(FLOAT + repr(obj).encode('ascii') + b'\n')

    def save_str(self, obj):
        if self.bin:
            encoded = obj.encode('utf-8', 'surrogatepass')
            size = len(encoded)
            if size <= 0xff and self.proto >= 4:
                self.write(SHORT_BINUNICODE + _pack('<B', size) + encoded)
            elif size > 0xffffffff and self.proto >= 4:
                self.write(BINUNICODE8 + _pack('<Q', size) + encoded)
            else:
                self.write(BINUNICODE + _pack('<I', size) + encoded)
        else:
            escaped = obj.replace('\\', '\\u005c')
            escaped = escaped.replace('\0', '\\u0000')
            escaped = escaped.replace('\n', '\\u000a')
            escaped = escaped.replace('\r', '\\u000d')
            escaped = escaped.replace('\x1a', '\\u001a')
            self.write(UNICODE + escaped.encode('raw-unicode-escape') + b'\n')
        self.memoize(obj)

    def save_bytes(self, obj):
        if self.proto < 3:
            if not obj:
                self.save_reduce(bytes, (), obj=obj)
            else:
                self.save_reduce(codecs.encode, (str(obj, 'latin1'), 'latin1'), obj=obj)
            return
        size = len(obj)
        if size <= 0xff:
            self.write(SHORT_BINBYTES + _pack('<B', size) + obj)
        elif size > 0xffffffff:
            self.write(BINBYTES8 + _pack('<Q', size) + obj)
        else:
            self.write(BINBYTES + _pack('<I', size) + obj)
        self.memoize(obj)

    def save_bytearray(self, obj):
        if self.proto < 5:
            if not obj:
                self.save_reduce(bytearray, (), obj=obj)
            else:
                self.save_reduce(bytearray, (bytes(obj),), obj=obj)
            return
        size = len(obj)
        self.write(BYTEARRAY8 + _pack('<Q', size) + bytes(obj))
        self.memoize(obj)

    def save_tuple(self, obj):
        if not obj:
            if self.bin:
                self.write(EMPTY_TUPLE)
            else:
                self.write(MARK + TUPLE)
            return
        size = len(obj)
        save = self.save
        if size <= 3 and self.proto >= 2:
            for element in obj:
                save(element)
            if self._key(obj) in self.memo:
                self.write(POP * size + self.get(self.memo[self._key(obj)][0]))
            else:
                self.write(_tuplesize2code[size])
                self.memoize(obj)
            return
        self.write(MARK)
        for element in obj:
            save(element)
        if self._key(obj) in self.memo:
            if self.bin:
                self.write(POP_MARK + self.get(self.memo[self._key(obj)][0]))
            else:
                self.write(POP * (size + 1) + self.get(self.memo[self._key(obj)][0]))
            return
        self.write(TUPLE)
        self.memoize(obj)

    def save_list(self, obj):
        if self.bin:
            self.write(EMPTY_LIST)
        else:
            self.write(MARK + LIST)
        self.memoize(obj)
        self._batch_appends(obj)

    def save_dict(self, obj):
        if self.bin:
            self.write(EMPTY_DICT)
        else:
            self.write(MARK + DICT)
        self.memoize(obj)
        self._batch_setitems(obj.items())

    def save_set(self, obj):
        if self.proto < 4:
            self.save_reduce(set, (list(obj),), obj=obj)
            return
        self.write(EMPTY_SET)
        self.memoize(obj)
        for batch in _batches(obj, 1000):
            self.write(MARK)
            for item in batch:
                self.save(item)
            self.write(ADDITEMS)

    def save_frozenset(self, obj):
        if self.proto < 4:
            self.save_reduce(frozenset, (list(obj),), obj=obj)
            return
        self.write(MARK)
        for item in obj:
            self.save(item)
        if self._key(obj) in self.memo:
            self.write(POP_MARK + self.get(self.memo[self._key(obj)][0]))
            return
        self.write(FROZENSET)
        self.memoize(obj)

    def _batch_appends(self, items):
        save = self.save
        write = self.write
        if not self.bin:
            for item in items:
                save(item)
                write(APPEND)
            return
        for batch in _batches(items, 1000):
            if len(batch) != 1:
                write(MARK)
                for item in batch:
                    save(item)
                write(APPENDS)
            else:
                save(batch[0])
                write(APPEND)

    def _batch_setitems(self, items):
        save = self.save
        write = self.write
        if not self.bin:
            for key, value in items:
                save(key)
                save(value)
                write(SETITEM)
            return
        for batch in _batches(items, 1000):
            if len(batch) != 1:
                write(MARK)
                for key, value in batch:
                    save(key)
                    save(value)
                write(SETITEMS)
            else:
                key, value = batch[0]
                save(key)
                save(value)
                write(SETITEM)

    def save_type(self, obj):
        if obj is type(None):
            return self.save_reduce(type, (None,), obj=obj)
        if obj is type(NotImplemented):
            return self.save_reduce(type, (NotImplemented,), obj=obj)
        if obj is type(...):
            return self.save_reduce(type, (...,), obj=obj)
        return self.save_global(obj)

    def save_global(self, obj, name=None):
        if name is None:
            resolved = _global_name(obj)
            if resolved is None:
                raise PicklingError('cannot pickle ' + repr(obj))
            module_name, name = resolved
        else:
            module_name = _whichmodule(obj, name)
            if module_name is None:
                raise PicklingError('global name does not refer to the object being pickled')
        if self.proto >= 4:
            self.save(module_name)
            self.save(name)
            self.write(STACK_GLOBAL)
        elif '.' in name:
            dotted = name.split('.')
            leaf = dotted.pop(0)
            for _ in dotted:
                self.save(getattr)
                if self.proto < 2:
                    self.write(MARK)
            self._toplevel(module_name, leaf)
            for attrname in dotted:
                self.save(attrname)
                self.write(TUPLE2 if self.proto >= 2 else TUPLE)
                self.write(REDUCE)
        else:
            self._toplevel(module_name, name)
        self.memoize(obj)

    def _toplevel(self, module_name, name):
        encoding = 'utf-8' if self.proto >= 3 else 'ascii'
        self.write(GLOBAL + module_name.encode(encoding) + b'\n')
        self.write(name.encode(encoding) + b'\n')

    def save_reduce(self, func, args, state=None, listitems=None, dictitems=None, state_setter=None, *, obj=None):
        if not callable(func):
            raise PicklingError('first item of the tuple returned by __reduce__ must be callable')
        if not isinstance(args, tuple):
            raise PicklingError('second item of the tuple returned by __reduce__ must be a tuple')
        func_name = getattr(func, '__name__', '')
        if self.proto >= 2 and func_name == '__newobj_ex__':
            cls, positional, keywords = args
            if not hasattr(cls, '__new__'):
                raise PicklingError('first argument to __newobj_ex__() has no __new__')
            if obj is not None and cls is not obj.__class__:
                raise PicklingError('first argument to __newobj_ex__() must be ' + repr(obj.__class__))
            if self.proto >= 4:
                self.save(cls)
                self.save(positional)
                self.save(keywords)
                self.write(NEWOBJ_EX)
            else:
                from functools import partial
                func = partial(cls.__new__, cls, *positional, **keywords)
                self.save(func)
                self.save(())
                self.write(REDUCE)
        elif self.proto >= 2 and func_name == '__newobj__':
            cls = args[0]
            if not hasattr(cls, '__new__'):
                raise PicklingError('first argument to __newobj__() has no __new__')
            if obj is not None and cls is not obj.__class__:
                raise PicklingError('first argument to __newobj__() must be ' + repr(obj.__class__))
            args = args[1:]
            self.save(cls)
            self.save(args)
            self.write(NEWOBJ)
        else:
            self.save(func)
            self.save(args)
            self.write(REDUCE)
        if obj is not None:
            if self._key(obj) in self.memo:
                self.write(POP + self.get(self.memo[self._key(obj)][0]))
            else:
                self.memoize(obj)
        if listitems is not None:
            self._batch_appends(listitems)
        if dictitems is not None:
            self._batch_setitems(dictitems)
        if state is not None:
            if state_setter is None:
                self.save(state)
                self.write(BUILD)
            else:
                self.save(state_setter)
                self.save(obj)
                self.save(state)
                self.write(TUPLE2)
                self.write(REDUCE)
                self.write(POP)


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
    return _Pickler(protocol).dump(obj)


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
            if op == 73 and number in ('00', '01'):
                stack.append(number == '01')
            else:
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
            stack.append(data[at:end].decode('raw-unicode-escape'))
            at = end + 1
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
        elif op == 48:
            stack.pop()
        elif op == 49:
            del stack[marks.pop():]
        elif op == 50:
            stack.append(stack[-1])
        elif op == 66:
            size = int.from_bytes(data[at:at + 4], 'little')
            at += 4
            stack.append(bytes(data[at:at + size]))
            at += size
        elif op == 67:
            size = data[at]
            at += 1
            stack.append(bytes(data[at:at + size]))
            at += size
        elif op == 129:
            args = stack.pop()
            cls = stack.pop()
            stack.append(cls.__new__(cls, *args))
        elif op == 141:
            size = int.from_bytes(data[at:at + 8], 'little')
            at += 8
            stack.append(data[at:at + size].decode('utf-8', 'surrogatepass'))
            at += size
        elif op == 142:
            size = int.from_bytes(data[at:at + 8], 'little')
            at += 8
            stack.append(bytes(data[at:at + size]))
            at += size
        elif op == 143:
            stack.append(set())
        elif op == 144:
            mark = marks.pop()
            stack[mark - 1].update(stack[mark:])
            del stack[mark:]
        elif op == 145:
            mark = marks.pop()
            value = frozenset(stack[mark:])
            del stack[mark:]
            stack.append(value)
        elif op == 146:
            keywords = stack.pop()
            positional = stack.pop()
            cls = stack.pop()
            stack.append(cls.__new__(cls, *positional, **keywords))
        elif op == 150:
            size = int.from_bytes(data[at:at + 8], 'little')
            at += 8
            stack.append(bytearray(data[at:at + size]))
            at += size
        else:
            raise UnpicklingError('unsupported pickle opcode: ' + str(op))
    raise UnpicklingError('pickle data was truncated')
