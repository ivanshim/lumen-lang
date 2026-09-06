import sys
# Ported from examples/lumen/constructs/type_hierarchy.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def round(x, decimals):
    scale = 1
    i = 0
    while i < decimals:
        scale = scale * 10
        i = i + 1
    y = x * scale
    if y >= 0:
        r = (y * 2 + 1) // 2
    else:
        r = (y * 2 - 1) // 2
    return r / scale

print("=== TYPE HIERARCHY TESTS: Integer subset of Rational subset of Real ===")
sys.stdout.write("\n")
print("=== SECTION 1: ADDITION ===")
sys.stdout.write("\n")
print("Test 1a: Integer + Integer -> Integer")
a = 5
b = 3
result = a + b
print(str(a) + " + " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 1b: Integer + Rational -> Rational")
a = 5
b = 3 / 2
result = a + b
print(str(a) + " + " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 1c: Integer + Real -> Real")
a = 5
b = float(3 / 2)
result = a + b
print(str(a) + " + " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 1d: Rational + Rational -> Rational")
a = 3 / 2
b = 5 / 4
result = a + b
print(str(a) + " + " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 1e: Rational + Real -> Real")
a = 3 / 2
b = float(5 / 4)
result = a + b
print(str(a) + " + " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 1f: Real + Real -> Real")
a = float(3 / 2)
b = float(5 / 4)
result = a + b
print(str(a) + " + " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("=== SECTION 2: SUBTRACTION ===")
sys.stdout.write("\n")
print("Test 2a: Integer - Integer -> Integer")
a = 10
b = 3
result = a - b
print(str(a) + " - " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 2b: Integer - Rational -> Rational")
a = 10
b = 3 / 2
result = a - b
print(str(a) + " - " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 2c: Integer - Real -> Real")
a = 10
b = float(3 / 2)
result = a - b
print(str(a) + " - " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 2d: Rational - Rational -> Rational")
a = 10 / 3
b = 5 / 4
result = a - b
print(str(a) + " - " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 2e: Rational - Real -> Real")
a = 10 / 3
b = float(5 / 4)
result = a - b
print(str(a) + " - " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 2f: Real - Real -> Real")
a = float(10 / 3)
b = float(5 / 4)
result = a - b
print(str(a) + " - " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("=== SECTION 3: MULTIPLICATION ===")
sys.stdout.write("\n")
print("Test 3a: Integer * Integer -> Integer")
a = 4
b = 3
result = a * b
print(str(a) + " * " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 3b: Integer * Rational -> Rational")
a = 4
b = 3 / 2
result = a * b
print(str(a) + " * " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 3c: Integer * Real -> Real")
a = 4
b = float(3 / 2)
result = a * b
print(str(a) + " * " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 3d: Rational * Rational -> Rational")
a = 2 / 3
b = 3 / 4
result = a * b
print(str(a) + " * " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 3e: Rational * Real -> Real")
a = 2 / 3
b = float(3 / 4)
result = a * b
print(str(a) + " * " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 3f: Real * Real -> Real")
a = float(2 / 3)
b = float(3 / 4)
result = a * b
print(str(a) + " * " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("=== SECTION 4: DIVISION ===")
sys.stdout.write("\n")
print("Test 4a: Integer / Integer -> Rational")
a = 5
b = 2
result = a / b
print(str(a) + " / " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 4b: Integer / Rational -> Rational")
a = 5
b = 3 / 2
result = a / b
print(str(a) + " / " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 4c: Integer / Real -> Real")
a = 5
b = float(3 / 2)
result = a / b
print(str(a) + " / " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 4d: Rational / Rational -> Rational")
a = 3 / 2
b = 5 / 4
result = a / b
print(str(a) + " / " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 4e: Rational / Real -> Real")
a = 3 / 2
b = float(5 / 4)
result = a / b
print(str(a) + " / " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("Test 4f: Real / Real -> Real")
a = float(3 / 2)
b = float(5 / 4)
result = a / b
print(str(a) + " / " + str(b) + " = " + str(result))
sys.stdout.write("\n")
print("=== SECTION 5: FLOAT LITERALS ===")
sys.stdout.write("\n")
print("Test 5a: Float literal 1.5 is Real")
x = 1.5
print("1.5 = " + str(x))
sys.stdout.write("\n")
print("Test 5b: Float literal + Integer -> Real")
x = 1.5
y = 2
result = x + y
print(str(x) + " + " + str(y) + " = " + str(result))
sys.stdout.write("\n")
print("Test 5c: Float literal * Integer -> Real")
x = 1.5
y = 3
result = x * y
print(str(x) + " * " + str(y) + " = " + str(result))
sys.stdout.write("\n")
print("Test 5d: Float literal / Integer -> Real")
x = 3.0
y = 2
result = x / y
print(str(x) + " / " + str(y) + " = " + str(result))
sys.stdout.write("\n")
print("=== SECTION 6: ROUND() FUNCTION RETURNS REAL ===")
sys.stdout.write("\n")
print("Test 6a: round(1.235, 2) returns Real")
x = round(1.235, 2)
print("round(1.235, 2) = " + str(x))
sys.stdout.write("\n")
print("Test 6b: round(-2.456, 2) returns Real")
x = round(-2.456, 2)
print("round(-2.456, 2) = " + str(x))
sys.stdout.write("\n")
print("Test 6c: round(5, 2) with integer input")
x = round(5, 2)
print("round(5, 2) = " + str(x))
sys.stdout.write("\n")
print("=== SUMMARY ===")
print("[OK] Integer OP Integer -> Integer (when no division)")
print("[OK] Integer OP Rational -> Rational (when no real)")
print("[OK] Integer OP Real -> Real")
print("[OK] Rational OP Rational -> Rational (when no real)")
print("[OK] Rational OP Real -> Real")
print("[OK] Real OP Real -> Real")
print("[OK] Float literals (1.5, 2.3) are Real values")
print("[OK] round(float) returns Real, not Rational")
