# tests/python/test_set.py:1093
if False:
    def setUp(self):
        self.enterContext(warnings_helper.check_warnings())
        warnings.simplefilter('ignore', BytesWarning)
        self.case   = "string and bytes set"
        self.values = ["a", "b", b"a", b"b"]
        self.set    = set(self.values)
        self.dup    = set(self.values)
        self.length = 4


print('read')
