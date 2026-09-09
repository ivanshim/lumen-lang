import unittest
from collections import namedtuple, deque
from copy import copy, deepcopy

class Trouble:
    def __init__(self, message):
        self.message = message

case = unittest.TestCase()
with case.assertRaisesRegex(Trouble, 'bad') as caught:
    raise Trouble('a bad value')
print(caught.exception.message)
with case.subTest(example=1):
    case.assertEqual(4, 4)

Point = namedtuple('Point', 'x y')
p = Point(2, 3)
q = copy(p)
q.x = 9
print(p.x, q.x, p.y)
queue = deque([1, 2])
queue.appendleft(0)
print(queue.popleft(), queue.pop())
