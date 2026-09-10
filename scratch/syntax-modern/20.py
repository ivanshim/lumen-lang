def side_effect():
    print("must not run")
    return None
@side_effect()
class C(side_effect()):
    print("body must not run")
    field: Missing = 1
