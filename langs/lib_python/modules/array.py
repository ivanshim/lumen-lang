# Numeric array elements are kept in a list.
typecodes = 'iBq'

class array:
    def __init__(self, typecode, initializer=None):
        if typecode not in ('i', 'B', 'q'):
            raise 'NotImplementedError: array supports only signed four- and eight-byte integers and unsigned bytes'
        self.typecode = typecode
        self.itemsize = 1 if typecode == 'B' else (8 if typecode == 'q' else 4)
        self.data = []
        if initializer is not None:
            self.extend(initializer)

    def append(self, value):
        if type(value) != type(1) and type(value) != type(True):
            raise 'TypeError: array item must be an integer'
        if self.typecode == 'B':
            if value < 0 or value > 255:
                raise OverflowError('unsigned byte integer is out of range')
        elif self.typecode == 'i' and (value < -2147483648 or value > 2147483647):
            raise 'OverflowError: signed integer is greater than maximum'
        self.data = [*self.data, int(value)]

    def fromfile(self, fileobj, count):
        data = fileobj.read(count * self.itemsize)
        if len(data) < count * self.itemsize:
            raise EOFError("read() didn't return enough bytes")
        for at in range(0, len(data), self.itemsize):
            piece = data[at:at + self.itemsize]
            if self.typecode == 'B':
                self.append(piece[0])
            else:
                self.append(int.from_bytes(piece, 'little', signed=True))

    def byteswap(self):
        if self.itemsize == 1:
            return
        swapped = []
        for value in self.data:
            piece = value.to_bytes(self.itemsize, 'little', signed=True)
            swapped = [*swapped, int.from_bytes(piece, 'big', signed=True)]
        self.data = swapped

    def __len__(self):
        return len(self.data)

    def __iter__(self):
        return iter(self.data)

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
            result += value.to_bytes(4, 'little', signed=True)
        return result

    def __int__(self):
        return int(self.tobytes())

    def __float__(self):
        return float(self.tobytes())
