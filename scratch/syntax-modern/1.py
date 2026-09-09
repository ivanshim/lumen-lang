try:
    raise ValueError
except* ValueError:
    print("group")
with (open("a") as f, open("b") as g):
    print("inside")
