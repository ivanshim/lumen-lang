namespace = {}
source = '\n'.join('value_%d = %d' % (i, i) for i in range(400))
source += '''
def read():
    return value_0, value_399, later
def write():
    global value_0
    value_0 = 17
class C:
    value_0 = 29
    def read(self, default=value_0):
        return default, value_0
later = 42
write()
'''
exec(source, namespace)
assert namespace['read']() == (17, 399, 42)
assert namespace['C']().read() == (29, 17)
exec('del value_399\nvalue_399 = 401', namespace)
assert namespace['read']() == (17, 401, 42)
assert eval('value_0 + value_399', namespace) == 418
print('global slots preserve reads, writes, defaults and dynamic namespaces')
