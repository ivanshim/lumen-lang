import unittest
class Quiet:
    def __str__(self):
        raise AssertionError('passing assertion formatted an operand')
    def __repr__(self):
        raise AssertionError('passing assertion represented an operand')
value=Quiet()
case=unittest.TestCase()
case.assertTrue(value)
case.assertFalse(False)
case.assertNotEqual(value,None)
case.assertIs(value,value)
case.assertIsNot(value,None)
case.assertIn(value,[value])
case.assertNotIn(None,[value])
case.assertIsInstance(value,Quiet)
case.assertNotIsInstance(value,str)
case.assertIsSubclass(Quiet,object)
case.assertNotIsSubclass(Quiet,str)
case.assertAlmostEqual(1.,1.)
case.assertNotAlmostEqual(1.,2.)
try:
    case.assertTrue(False,'detail')
except AssertionError as error:
    assert str(error)=='False is not true : detail'
else:
    raise AssertionError('false assertion passed')
print('passing assertions avoid message formatting; failing assertions retain diagnostics')
