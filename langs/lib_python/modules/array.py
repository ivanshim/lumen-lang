# Numeric array elements are kept in a list.
_binary_float = __math
typecodes = 'bBuhHiIlLqQfdw'

class array:
    def __init__(self, typecode, initializer=None):
        if not isinstance(typecode, str) or len(typecode) != 1:
            raise TypeError('array() argument 1 must be a unicode character')
        if typecode not in typecodes:
            raise ValueError('bad typecode')
        self.typecode = typecode
        self.itemsize = {'b': 1, 'B': 1, 'h': 2, 'H': 2, 'i': 4, 'I': 4, 'l': 8, 'L': 8, 'q': 8, 'Q': 8, 'u': 4, 'w': 4, 'f': 4, 'd': 8}[typecode]
        self.data = []
        if isinstance(initializer, (bytes, bytearray)):
            self.frombytes(initializer)
        elif initializer is not None:
            self.extend(initializer)

    def append(self, value):
        if self.typecode in ('u', 'w'):
            if not isinstance(value, str) or len(value) != 1:
                raise TypeError('array item must be unicode character')
        elif self.typecode in ('f', 'd'):
            if isinstance(value, (str, bytes, bytearray)):
                raise TypeError('must be real number, not ' + type(value).__name__)
            value = float(value)
            if self.typecode == 'f':
                value = _binary_float('bits_float32', _binary_float('float_bits32', value))
        else:
            import operator
            value = operator.index(value)
            bits = self.itemsize * 8
            signed = self.typecode.islower()
            low = -(2 ** (bits - 1)) if signed else 0
            high = 2 ** (bits - (1 if signed else 0)) - 1
            if value < low or value > high:
                raise OverflowError('array item is out of range')
        self.data.append(value)

    def extend(self, values):
        for value in values:
            self.append(value)

    def tolist(self):
        return list(self.data)

    def __getitem__(self, index):
        return self.data[index]

    def frombytes(self, data):
        with memoryview(data) as view:
            if not view.c_contiguous:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            raw = view.tobytes()
        if len(raw) % self.itemsize:
            raise ValueError('bytes length not a multiple of item size')
        import sys
        for at in range(0, len(raw), self.itemsize):
            value = int.from_bytes(raw[at:at + self.itemsize], sys.byteorder, signed=self.typecode.islower() and self.typecode not in ('u', 'w'))
            if self.typecode in ('f', 'd'):
                value = _binary_float('bits_float32' if self.typecode == 'f' else 'bits_float64', int.from_bytes(raw[at:at + self.itemsize], sys.byteorder))
            self.append(chr(value) if self.typecode in ('u', 'w') else value)

    def tobytes(self):
        if self.typecode in ('B', 'b'):
            return bytes([n & 255 for n in self.data])
        result = b''
        for value in self.data:
            number = ord(value) if self.typecode in ('u', 'w') else value
            if self.typecode in ('f', 'd'):
                number = _binary_float('float_bits32' if self.typecode == 'f' else 'float_bits64', value)
            result += number.to_bytes(self.itemsize, __import__('sys').byteorder, signed=self.typecode.islower() and self.typecode not in ('u', 'w', 'f', 'd'))
        return result

    def __int__(self):
        return int(self.tobytes())

    def __float__(self):
        return float(self.tobytes())

    def __len__(self):
        return len(self.data)

# As in CPython, numeric arrays implement the mutable sequence protocol.
from collections.abc import MutableSequence
MutableSequence.register(array)
