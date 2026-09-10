# tests/python/test_set.py:1555
if False:
    def test_difference_update_operator(self):
        try:
            self.set -= self.other
        except TypeError:
            pass
        else:
            self.fail("expected TypeError")


print('read')
