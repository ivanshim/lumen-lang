import inspect
import rlcompleter
for cls in (staticmethod, classmethod):
    try:
        inspect.signature(cls)
    except ValueError:
        pass
    else:
        raise AssertionError('builtin class has no signature')
assert rlcompleter.Completer({})._callable_postfix(staticmethod, 'staticmethod') == 'staticmethod('
print('class descriptors are not instance wrappers')
