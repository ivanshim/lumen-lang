def first(a: int, /, *, b: str = 'x') -> str:
    'adjacent ' 'docstring'
    return b

saved = first.__code__
assert first.__doc__ == 'adjacent docstring'
assert saved.co_consts[0] == 'adjacent docstring'
assert saved.co_posonlyargcount == 1
assert saved.co_kwonlyargcount == 1
assert first.__annotations__ == {'a': int, 'b': str, 'return': str}

def second():
    return 99

first.__code__ = second.__code__
assert saved.co_name == 'first'
assert saved.co_consts[0] == 'adjacent docstring'
assert first.__code__ is second.__code__
assert (lambda: 'expression, not a docstring').__doc__ is None

class Annotated:
    member: Annotated

assert Annotated.__annotations__['member'] is Annotated

async def coroutine():
    return 1

async def async_generator():
    yield 1

assert coroutine.__code__.co_flags & 128
assert async_generator.__code__.co_flags & 512
print('code identity, annotations, docstrings and coroutine flags preserved')

def annotated_closure():
    local = int
    def inner(value: local = 1) -> local:
        return value
    assert inner.__annotations__ == {'value': int, 'return': int}
    assert inner.__annotate__(1) == inner.__annotations__
annotated_closure()
