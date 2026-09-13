# A builtin reached through a value raises what the program code it ran
# raised, so that a handler standing around the call catches it.
run = exec
try:
    run("raise ValueError('boom')", {})
except ValueError as e:
    print("exec:", e)

arrange = sorted


def rank(item):
    raise ValueError("no order")


try:
    arrange([2, 1], key=rank)
except ValueError as e:
    print("sorted:", e)
