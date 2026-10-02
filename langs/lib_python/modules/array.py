# Numeric array elements are kept in a list.
typecodes = 'ibB'

class array:
    def __init__(self, typecode, initializer=None):
        if typecode not in ('i', 'b', 'B'):
            raise 'NotImplementedError: array supports only signed four-byte integers and unsigned bytes'
        self.typecode = typecode
        self.itemsize = 4 if typecode == 'i' else 1
        self.data = []
        if initializer is not None:
            if typecode == 'b' and isinstance(initializer, (bytes, bytearray)):
                initializer = [value if value < 128 else value - 256 for value in initializer]
            self.extend(initializer)

    def append(self, value):
        if self.typecode == 'b':
            import operator
            value = operator.index(value)
            if value < -9223372036854775808 or value > 9223372036854775807:
                raise OverflowError('Python int too large to convert to C long')
            if value < -128:
                raise OverflowError('signed char is less than minimum')
            if value > 127:
                raise OverflowError('signed char is greater than maximum')
        if type(value) != type(1) and type(value) != type(True):
            raise 'TypeError: array item must be an integer'
        if self.typecode == 'B':
            if value < 0 or value > 255:
                raise OverflowError('unsigned byte integer is out of range')
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

    def tobytes(self):
        if self.typecode == 'B':
            return bytes(self.data)
        result = b''
        for value in self.data:
            result += value.to_bytes(self.itemsize, 'little', signed=True)
        return result

    def __int__(self):
        return int(self.tobytes())

    def __float__(self):
        return float(self.tobytes())
