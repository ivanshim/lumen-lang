import sys
# Ported from examples/lumen/constructs/array_mutations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("=== Array Features Test ===")
arr = [10, 20, 30]
sys.stdout.write("Array: ")
print(arr)
sys.stdout.write("arr[0] = ")
print(arr[0])
arr2 = [1, 2, 3]
arr2[1] = 999
sys.stdout.write("After arr2[1]=999: ")
print(arr2)
arr3 = []
arr3.append(100)
arr3.append(200)
sys.stdout.write("After push: ")
print(arr3)
print("Done!")
