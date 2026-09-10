def deletion_forms():
    del a.b
    del A().b
    del a[:42, ..., :24:, 24, 100]
    del a[2:1024:10]
print("read")
