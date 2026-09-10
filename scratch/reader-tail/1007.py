# tests/python/test_opcodes.py:133
if False:
    def test_modulo_of_string_subclasses(self):
        class MyString(str):
            def __mod__(self, value):
                return 42
        self.assertEqual(MyString() % 3, 42)



print('read')
