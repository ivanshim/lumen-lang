# tests/python/test_set.py:1485
if False:
    def test_update_operator(self):
        try:
            self.set |= self.other
        except TypeError:
            pass
        else:
            self.fail("expected TypeError")


print('read')
