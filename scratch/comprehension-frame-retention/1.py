import sys

def inspect_parent(expected):
    caller = sys._getframe(1)
    assert caller.f_locals['i'] == expected
    nested = [j for j in range(2)]
    assert nested == [0, 1]
    assert caller.f_locals['i'] == expected
    return expected

def check():
    i = 'outer'
    result = [inspect_parent(i) for i in range(4)]
    assert result == [0, 1, 2, 3]
    assert i == 'outer'
    assert sys._getframe().f_locals['i'] == 'outer'
check()
print('comprehension frames retain and restore locals across calls')
