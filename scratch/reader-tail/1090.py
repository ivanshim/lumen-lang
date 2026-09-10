# tests/python/test_set.py:46
if False:
    def __hash__(self):
        self.hash_count += 1
        return int.__hash__(self)


print('read')
