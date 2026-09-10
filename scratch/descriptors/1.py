class Data:
    def __init__(self): self.writes = 0
    def __set_name__(self, owner, name): self.name = name
    def __get__(self, obj, owner):
        if obj is None: return self
        return self.writes
    def __set__(self, obj, value): self.writes += 1
class NonData:
    def __get__(self, obj, owner): return 'descriptor'
class C:
    data = Data()
    other = NonData()
c = C()
c.__dict__['data'] = 90
c.__dict__['other'] = 'instance'
c.data = 1
c.data = 2
print(c.data, c.other, C.data.name, C.data.writes)
