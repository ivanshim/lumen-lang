# Stub: hints carry no checks at run time and keep no supplied parameters.
class _Hint:
    def __getitem__(self, parameters):
        return self

Any = _Hint()
Optional = _Hint()
Union = _Hint()
List = _Hint()
Dict = _Hint()
Tuple = _Hint()
Set = _Hint()
Callable = _Hint()

class Generic:
    def __getitem__(cls, parameters):
        return cls

class Protocol:
    def __getitem__(cls, parameters):
        return cls

class TypeVar:
    def __init__(self, name, *constraints, **options):
        self.__name__ = name

    def __getitem__(self, parameters):
        return self

def cast(typ, value):
    return value

TYPE_CHECKING = False
