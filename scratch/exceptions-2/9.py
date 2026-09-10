g = ExceptionGroup("g", [ValueError("v"), TypeError("t")])
g.add_note("keep")
def wanted(e):
    return type(e) is ValueError
s = g.subgroup(wanted)
print(s.message, repr(s.__notes__), len(s.exceptions))
try:
    try:
        raise g
    except* ValueError:
        raise KeyError("new")
    except* TypeError:
        print("type handled")
except KeyError as e:
    print(str(e))
