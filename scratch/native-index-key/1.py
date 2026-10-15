class Key(int):
    def __hash__(self): return 101
key=Key(1)
class Mapping(dict): pass
for mapping in ({key:'value'}, Mapping({key:'value'})):
    assert mapping[key]=='value'
    try: mapping[1]
    except KeyError: pass
    else: raise AssertionError('dictionary key unwrapped')
assert 'abc'[key]=='b'
assert [4,5,6][key]==5
assert (4,5,6)[key]==5
print('mapping keys preserve subclass hashes; sequence indices unwrap')
