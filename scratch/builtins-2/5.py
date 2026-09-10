ns = {'n': 4}
exec(source='q = n + 1', globals=ns)
print(ns['q'])
print(eval(source='n * 3', globals=ns))
c = compile(source='n + 2', filename='<s>', mode='eval', flags=0, dont_inherit=True)
print(eval(c, ns))
e = eval
print(e('2 + 4'))
print(__name__, __doc__, '__file__' in globals(), '__builtins__' in globals())
print(hash(-1), hash(True), hash(123))
print(help(), breakpoint())
def f():
    v = 9
    print('v' in vars(), 'v' in dir())
f()
