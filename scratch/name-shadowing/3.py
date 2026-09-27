def report(name, f):
    try: print(name, f())
    except Exception as e: print(name, type(e).__name__)

def local():
    len = 1
    return len([])
report('local', local)

def parameter(len): return len([])
report('parameter', lambda: parameter(None))

def closure():
    len = 1
    def inner(): return len([])
    return inner()
report('closure', closure)

def body():
    class C:
        len = 1
        answer = len([])
    return C.answer
report('class', body)

max = 1
def global_call():
    global max
    return max(1)
report('global', global_call)

def caught():
    try: raise ValueError()
    except ValueError as len: return len([])
report('except', caught)

def imported():
    import math as len
    return len([])
report('import', imported)

def text():
    len = 'abs'
    return len(-1)
report('text', text)

def instance():
    class C: pass
    len = C()
    return len([])
report('instance', instance)

def effects():
    len = 1
    seen = []
    try: len(seen.append(3))
    except TypeError: return seen
report('arguments', effects)

def tail():
    len = object()
    def inner(): return len([])
    return inner()
report('tail', tail)
