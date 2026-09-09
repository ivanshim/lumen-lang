import sys
# Ported from examples/lumen/rsa_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("=== RSA Cryptography Demonstration ===")
print("")
print("Step 1: Key Generation")
print("----------------------")
p = 61
q = 53
sys.stdout.write("Selected prime p = ")
print(p)
sys.stdout.write("Selected prime q = ")
print(q)
if not is_prime(n= p):
    sys.exit("p must be prime")
if not is_prime(n= q):
    sys.exit("q must be prime")
if p == q:
    sys.exit("p and q must be distinct")
print("")
n = p * q
sys.stdout.write("Computed n = p * q = ")
print(n)
print("")
phi = (p - 1) * (q - 1)
sys.stdout.write("Computed phi(n) = (p-1)(q-1) = ")
print(phi)
print("")
e = 17
sys.stdout.write("Selected public exponent e = ")
print(e)
if e <= 1 or e >= phi:
    sys.exit("e must satisfy 1 < e < phi(n)")
if not is_coprime(a= e, b= phi):
    sys.exit("e must be coprime to phi(n)")
sys.stdout.write("Verified gcd(e, phi(n)) = ")
print(gcd(a= e, b= phi))
print("")
d = mod_inverse(a= e, m= phi)
sys.stdout.write("Computed private exponent d = ")
print(d)
verification = (d * e) % phi
sys.stdout.write("Verification: d * e mod phi(n) = ")
print(verification)
if verification != 1:
    sys.exit("Private key computation failed: d * e mod phi(n) must equal 1")
print("")
print("=== Generated Keys ===")
sys.stdout.write("Public key:  (e, n) = (")
sys.stdout.write(str(e))
sys.stdout.write(", ")
sys.stdout.write(str(n))
print(")")
sys.stdout.write("Private key: (d, n) = (")
sys.stdout.write(str(d))
sys.stdout.write(", ")
sys.stdout.write(str(n))
print(")")
print("")
print("=== Encryption and Decryption ===")
print("")
m = 42
sys.stdout.write("Original message m = ")
print(m)
if m < 0 or m >= n:
    sys.exit("Message must be in range 0 to n-1")
cipher = mod_pow(base= m, exp= e, m= n)
sys.stdout.write("Encrypted cipher c = m^e mod n = ")
print(cipher)
decrypted = mod_pow(base= cipher, exp= d, m= n)
sys.stdout.write("Decrypted message m' = c^d mod n = ")
print(decrypted)
if decrypted == m:
    print("SUCCESS: Decryption successful: m' = m")
else:
    sys.exit("Decryption failed")
print("")
print("=== Digital Signature ===")
print("")
message_to_sign = 100
sys.stdout.write("Message to sign = ")
print(message_to_sign)
if message_to_sign < 0 or message_to_sign >= n:
    sys.exit("Message to sign must be in range 0 to n-1")
signature = mod_pow(base= message_to_sign, exp= d, m= n)
sys.stdout.write("Digital signature s = m^d mod n = ")
print(signature)
verified_message = mod_pow(base= signature, exp= e, m= n)
sys.stdout.write("Verified message m' = s^e mod n = ")
print(verified_message)
if verified_message == message_to_sign:
    print("SUCCESS: Signature verification successful")
else:
    sys.exit("Signature verification failed")
print("")
print("=== Summary ===")
print("")
print("RSA Properties Demonstrated:")
print("1. Key Generation: Generated (e,n) and (d,n) from primes p and q")
print("2. Encryption: c = m^e mod n")
print("3. Decryption: m = c^d mod n")
print("4. Digital Signature: s = m^d mod n")
print("5. Signature Verification: m = s^e mod n")
print("")
print("Mathematical Correctness:")
print("  (m^e)^d == m^(ed) == m^1 == m (mod n)")
print("  because ed == 1 (mod phi(n)) by construction")
print("")
print("Security Note: This demo uses 6-bit primes. Real RSA requires")
print("              at least 2048-bit primes for security.")
