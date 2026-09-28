# Numeric array elements are kept in a list.
typecodes = 'iB'

class array:
    def __init__(self, typecode, initializer=None):
        if typecode not in ('i', 'B'):
            raise 'NotImplementedError: array supports only signed four-byte integers and unsigned bytes'
        self.typecode = typecode
        self.itemsize = 1 if typecode == 'B' else 4
        self.data = []
        if initializer is not None:
            self.extend(initializer)

    def append(self, value):
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
        raise 'NotImplementedError: array byte buffers are not supported'
