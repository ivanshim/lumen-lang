# Read the class annotation without asking for class construction.
def define_class():
    class C:
        field: Missing = 1

print("read class annotation")
