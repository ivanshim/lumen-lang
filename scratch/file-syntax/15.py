def f(self, err):
    self.assertEqual(err.filename, "<testcase>")
    return err.exception.lineno
values = []
values.append(5)
print(values[0])
