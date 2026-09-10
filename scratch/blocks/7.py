class Manager:
    def __init__(self, value):
        self.value = value
    def __enter__(self):
        return self.value
    def __exit__(self, kind, value, trace):
        return False

with Manager(5) as x, Manager(6) as y:
    print(x, y)
with (Manager(7) as x, Manager(8) as y,):
    print(x, y)
with Manager(1), Manager(2):
    print(3)
with Manager([4, 5]) as (x, y):
    print(x, y)
