# tests/python/test_set.py:1339
if False:
    def test_remove_absent(self):
        try:
            self.set.remove("d")
            self.fail("Removing missing element should have raised LookupError")
        except LookupError:
            pass


print('read')
