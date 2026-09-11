class Manager:
    def __enter__(self):
        print("enter")
        return self
    def __exit__(self, kind, value, trace):
        print("exit")
        return False
class BadEnter(Manager):
    def __enter__(self):
        print("bad enter")
        raise ValueError("bad")
try:
    with Manager(), BadEnter():
        print("wrong")
except ValueError:
    print("caught")
for i in range(2):
    with Manager():
        try:
            break
        finally:
            print("finally")
else:
    print("wrong")
print("done")
