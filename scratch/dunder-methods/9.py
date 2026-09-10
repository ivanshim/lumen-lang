class BadTruth:
    def __bool__(self):
        return 1
print(bool(BadTruth()))
