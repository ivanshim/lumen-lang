import sys
# Ported from examples/lumen/constructs/integer_quotient.lm by scripts/port_examples.py; edit the Lumen original, not this file.
sys.stdout.write("=== INTEGER QUOTIENT (//) OPERATOR TESTS ===\n")
sys.stdout.write("\n")
sys.stdout.write("=== SECTION 1: Integer // Integer = Integer ===\n")
sys.stdout.write("Test 1a: 17 // 5 = ")
a = 17
b = 5
result = a // b
print(result)
sys.stdout.write("Check identity: 17 == 5 * (17 // 5) + (17 % 5) = ")
check = b * result + (a % b)
print(check == a)
sys.stdout.write("\n")
sys.stdout.write("Test 1b: -17 // 5 = ")
a = -17
b = 5
result = a // b
print(result)
sys.stdout.write("Check identity: -17 == 5 * (-17 // 5) + (-17 % 5) = ")
check = b * result + (a % b)
print(check == a)
sys.stdout.write("\n")
sys.stdout.write("Test 1c: 17 // -5 = ")
a = 17
b = -5
result = a // b
print(result)
sys.stdout.write("Check identity: 17 == -5 * (17 // -5) + (17 % -5) = ")
check = b * result + (a % b)
print(check == a)
sys.stdout.write("\n")
sys.stdout.write("Test 1d: -17 // -5 = ")
a = -17
b = -5
result = a // b
print(result)
sys.stdout.write("Check identity: -17 == -5 * (-17 // -5) + (-17 % -5) = ")
check = b * result + (a % b)
print(check == a)
sys.stdout.write("\n")
sys.stdout.write("=== SECTION 2: Rational // Integer = Rational ===\n")
sys.stdout.write("Test 2a: (17/3) // 2 = ")
a = 17 / 3
b = 2
result = a // b
print(result)
sys.stdout.write("(17/3 = 5.666..., quotient is 2)\n")
sys.stdout.write("\n")
sys.stdout.write("Test 2b: (20/3) // 2 = ")
a = 20 / 3
b = 2
result = a // b
print(result)
sys.stdout.write("(20/3 = 6.666..., quotient is 3)\n")
sys.stdout.write("\n")
sys.stdout.write("Test 2c: (-17/3) // 2 = ")
a = -17 / 3
b = 2
result = a // b
print(result)
sys.stdout.write("(-17/3 = -5.666..., quotient truncates to -5)\n")
sys.stdout.write("\n")
sys.stdout.write("=== SECTION 3: Rational // Rational = Rational ===\n")
sys.stdout.write("Test 3a: (17/3) // (5/2) = ")
a = 17 / 3
b = 5 / 2
result = a // b
print(result)
sys.stdout.write("(17/3 / 5/2 = 34/15 = 2.266..., quotient is 2)\n")
sys.stdout.write("\n")
sys.stdout.write("Test 3b: (20/3) // (3/2) = ")
a = 20 / 3
b = 3 / 2
result = a // b
print(result)
sys.stdout.write("(20/3 / 3/2 = 40/9 = 4.444..., quotient is 4)\n")
sys.stdout.write("\n")
sys.stdout.write("=== SECTION 4: Float Literals (Real) // Integer = Real ===\n")
sys.stdout.write("Test 4a: 3.5 // 2 = ")
a = 3.5
b = 2
result = a // b
print(result)
sys.stdout.write("\n")
sys.stdout.write("Test 4b: 5.7 // 3 = ")
a = 5.7
b = 3
result = a // b
print(result)
sys.stdout.write("\n")
sys.stdout.write("Test 4c: -3.5 // 2 = ")
a = -3.5
b = 2
result = a // b
print(result)
sys.stdout.write("\n")
sys.stdout.write("=== SECTION 5: Edge Cases ===\n")
sys.stdout.write("Test 5a: 0 // 5 = ")
result = 0 // 5
print(result)
sys.stdout.write("\n")
sys.stdout.write("Test 5b: 7 // 1 = ")
result = 7 // 1
print(result)
sys.stdout.write("\n")
sys.stdout.write("Test 5c: -7 // 1 = ")
result = -7 // 1
print(result)
sys.stdout.write("\n")
sys.stdout.write("Test 5d: Verify truncation toward zero: 5 // 2 = ")
result = 5 // 2
print(result)
sys.stdout.write("(not floor division which would be 2.5 -> 2, but truncate 2.5 -> 2) [OK]\n")
sys.stdout.write("\n")
sys.stdout.write("Test 5e: Verify truncation toward zero: -5 // 2 = ")
result = -5 // 2
print(result)
sys.stdout.write("(not floor division which would be -2.5 -> -3, but truncate -2.5 -> -2) [OK]\n")
sys.stdout.write("\n")
sys.stdout.write("=== SECTION 6: Operator Precedence (same as * / %) ===\n")
sys.stdout.write("Test 6a: 10 + 3 // 2 should be 10 + 1 = 11: ")
result = 10 + 3 // 2
print(result)
sys.stdout.write("\n")
sys.stdout.write("Test 6b: 20 // 3 * 2 should be (20 // 3) * 2 = 6 * 2 = 12: ")
result = 20 // 3 * 2
print(result)
sys.stdout.write("\n")
sys.stdout.write("=== SUMMARY ===\n")
sys.stdout.write("[OK] Integer // Integer returns Integer\n")
sys.stdout.write("[OK] Rational // Integer returns Rational\n")
sys.stdout.write("[OK] Rational // Rational returns Rational\n")
sys.stdout.write("[OK] Real // ... returns Real\n")
sys.stdout.write("[OK] Truncates toward zero (not floor division)\n")
sys.stdout.write("[OK] Identity a == b * (a // b) + (a % b) holds\n")
sys.stdout.write("[OK] Division by zero raises error\n")
