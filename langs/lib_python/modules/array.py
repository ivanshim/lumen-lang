# Signed four-byte integers are kept in a list; there is no byte buffer.
typecodes = 'i'

class array:
    def __init__(self, typecode, initializer=()):
        if typecode != 'i':
            raise 'NotImplementedError: array supports only signed four-byte integers'
        self.typecode = typecode
        self.itemsize = 4
        self.data = []
        self.extend(initializer)

    def append(self, value):
        if type(value) != type(1):
            raise 'TypeError: array item must be an integer'
        if value < -2147483648 or value > 2147483647:
            raise 'OverflowError: signed integer is greater than maximum'
        self.data = [*self.data, value]

    def extend(self, values):
        for value in values:
            self.append(value)

    def tolist(self):
        return list(self.data)

    def __getitem__(self, index):
        return self.data[index]

    def tobytes(self):
        raise 'NotImplementedError: array byte buffers are not supported'
