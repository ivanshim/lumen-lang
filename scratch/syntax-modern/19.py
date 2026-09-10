def outer():
    if False:
        class C:
            def suspended(self):
                yield 1
            field: Missing = 2
    return 3
print(outer())
