def identity(x): return x

@identity(identity)(identity)
def answer(): return 42

print(identity(identity)(answer)())
print(identity([answer])[0]())
print([identity][0](identity)(answer)())
print((identity)(answer)())

def maker():
    print("callee")
    return identity

def argument():
    print("argument")
    return 7

print(maker()(argument()))

def decorated_methods():
    @[identity][0].__call__.__call__
    def f(): pass

print("read")
