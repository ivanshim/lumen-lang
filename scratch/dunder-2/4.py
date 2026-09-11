class I:
    def __index__(self):
        return "x"
print([10][I()])
