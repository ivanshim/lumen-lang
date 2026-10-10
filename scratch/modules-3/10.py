from collections import OrderedDict
from types import MappingProxyType
import inspect
from dataclasses import dataclass

class Mapping:
    def __getitem__(self, key):
        return {'item': 3}[key]
    def __iter__(self):
        return iter(['item'])
    def __len__(self):
        return 1
    def __bool__(self):
        return False

for mapping in ({'item': 3}, OrderedDict(item=3), Mapping()):
    proxy = MappingProxyType(mapping)
    assert len(proxy) == 1
    assert bool(proxy) and not (not proxy)
    assert list(iter(proxy)) == ['item']
    assert proxy['item'] == 3
    print(type(mapping).__name__, 'proxy ok')

mapping = OrderedDict()
proxy = MappingProxyType(mapping)
assert not proxy and not bool(proxy)
mapping['later'] = 5
assert proxy and bool(proxy) and next(iter(proxy)) == 'later'
del mapping['later']
assert not proxy and not bool(proxy) and list(iter(proxy)) == []

@dataclass
class Record:
    self: str
record = Record('value')
parameters = inspect.signature(Record.__init__).parameters
assert parameters and bool(parameters)
assert list(iter(parameters)) == ['__dataclass_self__', 'self']
parameters = inspect.signature(None.__bool__).parameters
assert parameters and list(iter(parameters)) == ['self']
print('live mappings and signatures ok')

from rlcompleter import Completer
matches = Completer({}).attr_matches('None.')
for name in ('__bool__', '__dir__', '__init__', '__new__', '__subclasshook__'):
    assert 'None.' + name + '(' in matches
print('completion parameters ok')
