# tests/python/test_set.py:1753
if False:
    def __next__(self):
        if self.i >= len(self.seqn): raise StopIteration
        v = self.seqn[self.i]
        self.i += 1
        return v


print('read')
