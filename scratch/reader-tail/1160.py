# tests/python/test_set.py:910
if False:
    def check_repr_against_values(self):
        text = repr(self.set)
        self.assertStartsWith(text, '{')
        self.assertEndsWith(text, '}')

        result = text[1:-1].split(', ')
        result.sort()
        sorted_repr_values = [repr(value) for value in self.values]
        sorted_repr_values.sort()
        self.assertEqual(result, sorted_repr_values)


print('read')
