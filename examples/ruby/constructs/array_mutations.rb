# Ported from examples/lumen/constructs/array_mutations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts("=== Array Features Test ===")
arr = [10, 20, 30]
print("Array: ")
puts(arr)
print("arr[0] = ")
puts(arr[0])
arr2 = [1, 2, 3]
arr2[1] = 999
print("After arr2[1]=999: ")
puts(arr2)
arr3 = []
arr3.push(100)
arr3.push(200)
print("After push: ")
puts(arr3)
puts("Done!")
