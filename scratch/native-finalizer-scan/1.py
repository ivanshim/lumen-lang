import gc
import weakref
events = []
class Item:
    def __init__(self, number): self.number = number
    def __del__(self): events.append((self.number, id(self)))
owners = [Item(number) for number in range(9)]
identities = [id(item) for item in owners]
references = [weakref.ref(item) for item in owners]
dropped = owners.pop(4)
del dropped
assert events == [(4, identities[4])]
assert references[4]() is None
del owners[0]
assert events[-1] == (0, identities[0])
del owners[-1]
assert events[-1] == (8, identities[8])
assert all(reference() is not None for index, reference in enumerate(references) if index not in (0, 4, 8))
owners.clear()
gc.collect()
assert sorted(number for number, identity in events) == list(range(9))
assert all(identity == identities[number] for number, identity in events)
assert all(reference() is None for reference in references)
print('finalizers preserve prompt release, identity and retained owners')
