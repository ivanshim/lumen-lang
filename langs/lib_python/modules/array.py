# Numeric array elements are kept in a list.
typecodes = 'ibB'

class array:
    def __init__(self, typecode, initializer=None):
        if typecode not in ('i', 'b', 'B'):
            raise 'NotImplementedError: array supports only signed four-byte integers and unsigned bytes'
        self.typecode = typecode
        self.itemsize = 1 if typecode in ('b', 'B') else 4
        self.data = []
        if isinstance(initializer, (bytes, bytearray)):
            self.frombytes(initializer)
        elif initializer is not None:
            self.extend(initializer)

    def append(self, value):
        if type(value) != type(1) and type(value) != type(True):
            raise 'TypeError: array item must be an integer'
        if self.typecode == 'B':
            if value < 0 or value > 255:
                raise OverflowError('unsigned byte integer is out of range')
        elif self.typecode == 'b':
            if value < -128 or value > 127:
                raise OverflowError('signed char is out of range')
        elif value < -2147483648 or value > 2147483647:
            raise 'OverflowError: signed integer is greater than maximum'
        self.data = [*self.data, int(value)]

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
            self.append(int.from_bytes(raw[at:at + self.itemsize], sys.byteorder, signed=self.typecode != 'B'))

    def tobytes(self):
        if self.typecode in ('B', 'b'):
            return bytes([n & 255 for n in self.data])
        result = b''
        for value in self.data:
            result += value.to_bytes(4, __import__('sys').byteorder, signed=True)
        return result

    def __int__(self):
        return int(self.tobytes())

    def __float__(self):
        return float(self.tobytes())

    def __len__(self):
        return len(self.data)
