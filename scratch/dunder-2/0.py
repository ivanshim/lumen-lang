class I:
    def __index__(self):
        return 2
print([10, 20, 30][I()], bin(I()), range(5)[I()])
