import copy

def f(self):
    return self.value

class C:
    alias = f

assert not hasattr(C, 'f')
owner = C()
owner.value = [[31408]]
original = owner.alias
method, copied_owner = copy.deepcopy([original, owner])
assert method.__func__ is f
assert method.__self__ is copied_owner
assert copied_owner is not owner
assert method() == [[31408]]
assert method() is copied_owner.value
assert copied_owner.value is not owner.value
assert copied_owner.value[0] is not owner.value[0]
copied_owner.value[0].append(8)
assert method() == [[31408, 8]]
assert owner.value == [[31408]]
print('Aliased bound methods retain their function and a deeply copied owner')
