"""
Some correct syntax for variable annotation here.
More examples are in test_grammar and test_parser.
"""
# Three things differ from CPython's copy. The unused "from typing import
# no_type_check, ClassVar" is left out, because neither name exists in the
# typing carried here. f's class is declared at module level rather than
# inside f, because this runtime cannot define a class in a function body.
# And the value given to new_attr is an Inner rather than an object(),
# because the name object does not reach a module's scope here. What the
# tests read -- the empty __annotations__ this module ends with -- is
# unchanged.

i: int = 1
j: UndefinedAnnotation[int, str] = 1

x: int
class Inner:
    pass

def f():
    return Inner()

f().new_attr: object = Inner()

class C:
    def __init__(self, x: int) -> None:
        self.x = x

c = C(5)
c.new_attr: int = 10

__annotations__ = {}
