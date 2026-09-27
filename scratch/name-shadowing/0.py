def report(name, f):
    try: print(name, f())
    except Exception as e: print(name, type(e).__name__)

def case_nested_def():
    def len(a, b): return a + b
    return len(1, 2)
report("nested_def", case_nested_def)

def case_parameter(len=lambda a, b: a + b):
    return len(1, 2)
report("parameter", case_parameter)

def case_assignment():
    len = lambda a, b: a + b
    return len(2, 3)
report("assignment", case_assignment)

def case_late_assignment():
    return len([])
    len = 1
report("late_assignment", case_late_assignment)

def case_late_def():
    return len([])
    def len(a): return a
report("late_def", case_late_def)

def case_delete():
    del len
    return len([])
report("delete", case_delete)

def case_import():
    from math import sqrt as len
    return len(9)
report("import", case_import)

def case_late_import():
    return len([])
    from math import sqrt as len
report("late_import", case_late_import)

def case_for():
    for len in [lambda a: a + 1]:
        pass
    return len(4)
report("for", case_for)

def case_late_for():
    return len([])
    for len in []: pass
report("late_for", case_late_for)

def case_walrus():
    (len := lambda a: a + 2)
    return len(4)
report("walrus", case_walrus)

def case_late_walrus():
    return len([])
    (len := 1)
report("late_walrus", case_late_walrus)

def case_except():
    try: raise ValueError()
    except ValueError as len:
        return callable(len)
report("except", case_except)

def case_late_except():
    return len([])
    try: pass
    except ValueError as len: pass
report("late_except", case_late_except)

def case_closure():
    def len(a): return a + 3
    def inner(): return len(5)
    return inner()
report("closure", case_closure)

def case_nonlocal():
    len = lambda a: a + 4
    def inner():
        nonlocal len
        return len(5)
    return inner()
report("nonlocal", case_nonlocal)

def case_comprehension():
    return [len(5) for len in [lambda a: a + 5]]
report("comprehension", case_comprehension)

def case_lambda():
    return (lambda len: len(6))(lambda a: a + 6)
report("lambda", case_lambda)

def case_class():
    class C:
        def len(a): return a + 7
        answer = len(6)
    return C.answer
report("class", case_class)

def case_class_fallback():
    class C:
        answer = len([1])
        len = 5
    return C.answer
report("class_fallback", case_class_fallback)

def case_class_method():
    class C:
        len = 5
        def method(self): return len([1, 2])
    return C().method()
report("class_method", case_class_method)

def case_with():
    class C:
        def __enter__(self): return lambda a: a + 8
        def __exit__(self, *args): pass
    with C() as len:
        return len(6)
report("with", case_with)

def case_late_with():
    return len([])
    with None as len: pass
report("late_with", case_late_with)

def global_reader():
    global sum
    return sum(3, 4)
sum = lambda a, b: a + b
report("global", global_reader)

def late_global_reader(): return max(3, 4)
max = lambda a, b: a + b
report("late_global", late_global_reader)

def global_writer():
    global min
    min = lambda a: a + 9
def global_user(): return min(7)
global_writer()
report("global_write", global_user)

def deleted_local():
    len = lambda a: a
    del len
    return len([])
report("deleted_local", deleted_local)

def class_deleted():
    class C:
        len = lambda a: a
        del len
        answer = len([1, 2])
    return C.answer
report("class_deleted", class_deleted)

def class_conditional():
    class C:
        if False: len = lambda a: a
        answer = len([1, 2, 3])
    return C.answer
report("class_conditional", class_conditional)

def local_super():
    super = lambda a: a + 1
    return super(7)
report("local_super", local_super)

def local_bytes():
    bytes = lambda a: a + 2
    f = bytes
    return f(7)
report("local_bytes", local_bytes)

def later_closure():
    def inner(): return len(8)
    len = lambda a: a + 3
    return inner()
report("later_closure", later_closure)

def comprehension_walrus():
    values = [(len := lambda a: a + 4) for x in [1]]
    return len(8)
report("comprehension_walrus", comprehension_walrus)

def class_before_local():
    len = lambda a: 99
    class C:
        before = len([1, 2])
        len = lambda a: 7
        after = len([])
    return C.before, C.after
report("class_before_local", class_before_local)

def class_deleted_outer():
    len = lambda a: 99
    class C:
        len = lambda a: 7
        del len
        answer = len([1, 2])
    return C.answer
report("class_deleted_outer", class_deleted_outer)

def class_free():
    len = lambda a: 99
    class C:
        answer = len([])
    return C.answer
report("class_free", class_free)

def class_annotated():
    len = lambda a: 99
    class C:
        len: int
        answer = len([1, 2])
    return C.answer
report("class_annotated", class_annotated)

def class_namespace():
    class C:
        locals()['len'] = lambda a: 42
        answer = len([])
    return C.answer
report("class_namespace", class_namespace)

def local_generator():
    def sum(a, b):
        yield a + b
    return list(sum(2, 3))
report("local_generator", local_generator)

def class_nonlocal():
    len = lambda a: 12
    class C:
        nonlocal len
        answer = len([])
    return C.answer
report("class_nonlocal", class_nonlocal)

def annotated_local():
    len: int
    return len([])
report("annotated_local", annotated_local)

def builtin_scope():
    def unrelated():
        len = 3
    return len([1, 2, 3])
report("builtin_scope", builtin_scope)

def builtin_after_comprehension():
    values = [len([]) for len in [lambda a: 8]]
    return values, len([1, 2])
report("builtin_after_comprehension", builtin_after_comprehension)

def import_module():
    import math as len
    return len.sqrt(16)
report("import_module", import_module)

def deleted_exception():
    try: raise ValueError()
    except ValueError as len: pass
    return len([])
report("deleted_exception", deleted_exception)

def augmented_local():
    len += 1
    return len([])
report("augmented_local", augmented_local)

def local_class():
    class len:
        def __init__(self, a): self.a = a
    return len(12).a
report("local_class", local_class)
