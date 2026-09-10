def read_forms():
    int.new_attr: int
    [list][0]: type
    my_lst[one()-1]: int = 5
    no_name[does_not_exist]: no_name_again = 1/0
    no_name[does_not_exist]: 1/0 = 0
    a.b: int = (1, 2)
    class C:
        __foo: int
        s: str = "attr"
        def __init__(self, x):
            self.x: int = x
    class CBad:
        no_such_name_defined.attr: int = 0
    class Cbad2(C):
        x: int
        x.y: list = []
print("read forms")
