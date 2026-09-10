# tests/python/test_set.py:614
if False:
    def test_rich_compare(self):
        class TestRichSetCompare:
            def __gt__(self, some_set):
                self.gt_called = True
                return False
            def __lt__(self, some_set):
                self.lt_called = True
                return False
            def __ge__(self, some_set):
                self.ge_called = True
                return False
            def __le__(self, some_set):
                self.le_called = True
                return False

        # This first tries the builtin rich set comparison, which doesn't know
        # how to handle the custom object. Upon returning NotImplemented, the
        # corresponding comparison on the right object is invoked.
        myset = {1, 2, 3}

        myobj = TestRichSetCompare()
        myset < myobj
        self.assertTrue(myobj.gt_called)

        myobj = TestRichSetCompare()
        myset > myobj
        self.assertTrue(myobj.lt_called)

        myobj = TestRichSetCompare()
        myset <= myobj
        self.assertTrue(myobj.ge_called)

        myobj = TestRichSetCompare()
        myset >= myobj
        self.assertTrue(myobj.le_called)


print('read')
