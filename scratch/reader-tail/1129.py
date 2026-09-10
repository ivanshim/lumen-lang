# tests/python/test_set.py:452
if False:
    def test_remove_keyerror_set(self):
        key = self.thetype([3, 4])
        try:
            self.s.remove(key)
        except KeyError as e:
            self.assertTrue(e.args[0] is key,
                         "KeyError should be {0}, not {1}".format(key,
                                                                  e.args[0]))
        else:
            self.fail()


print('read')
