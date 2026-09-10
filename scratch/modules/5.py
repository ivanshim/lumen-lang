import unittest
from collections import namedtuple, deque
from copy import copy, deepcopy

case = unittest.TestCase()
with case.assertRaisesRegex(case.failureException, '!='):
    case.assertEqual(1, 2)
print('caught')
with case.subTest(example=1):
    case.assertEqual(4, 4)

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

p = Point(2, 3)
q = copy(p)
q.x = 9
print(p.x, q.x, p.y)
Pair = namedtuple('Pair', 'x y')
pair = Pair(4, 5)
print(pair.x, pair.y)
queue = deque([1, 2])
queue.appendleft(0)
print(queue.popleft(), queue.pop())

from test import support
with support.captured_stdout() as stream:
    print('kept')
print(stream.getvalue(), end='')
