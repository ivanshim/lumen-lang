# tests/python/test_set.py:1507
if False:
    def test_intersection_update_operator(self):
        try:
            self.set &= self.other
        except TypeError:
            pass
        else:
            self.fail("expected TypeError")


print('read')
