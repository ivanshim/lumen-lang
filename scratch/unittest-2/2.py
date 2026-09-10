import unittest

def first_line(text):
    line = ''
    for index in range(len(text)):
        character = text[index]
        if character == '\n':
            break
        line += character
    return line

case = unittest.TestCase()
with case.assertRaises(case.failureException) as lists:
    case.assertEqual([1, 2], [1, 3])
print(first_line(str(lists.exception)))
with case.assertRaises(case.failureException) as lines:
    case.assertEqual('first\nsecond\n', 'first\nthird\n')
print(first_line(str(lines.exception)))
