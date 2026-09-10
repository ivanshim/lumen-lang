def deco(value):
    print('decorate class')
    return value
@deco
class C:
    pass
c = C()
print('made')
