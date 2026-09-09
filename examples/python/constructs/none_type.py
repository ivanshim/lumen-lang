# Ported from examples/lumen/constructs/none_type.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def no_return():
    print("This function returns null implicitly")

def explicit_null():
    print("Returning null explicitly")
    return None

def conditional_null(x):
    if x < 0:
        return None
    else:
        return x * 2

print("Test: null Type")
print("Calling no_return():")
result1 = no_return()
print(result1)
print("Calling explicit_null():")
result2 = explicit_null()
print(result2)
print("conditional_null(5):")
print(conditional_null(x= 5))
print("conditional_null(-3):")
print(conditional_null(x= -3))
x = None
print("let x = null:")
print(x)
def check_value(val):
    if val == None:
        print("Value is null")
    else:
        print("Value is not null")

check_value(val= None)
check_value(val= 42)
check_value(val= "hello")
