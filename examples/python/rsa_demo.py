import sys
# Ported from examples/lumen/rsa_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def mod_pow(base, exp, m):
    result = 1
    base = base % m
    while exp > 0:
        if exp % 2 == 1:
            result = (result * base) % m
        exp = exp // 2
        base = (base * base) % m
    return result

def gcd(a, b):
    while b != 0:
        t = b
        b = a % b
        a = t
    return a

def extended_gcd(a, b):
    if b == 0:
        return [a, 1, 0]
    else:
        r = extended_gcd(b, a % b)
        g = r[0]
        x = r[2]
        y = r[1] - (a // b) * r[2]
        return [g, x, y]

def is_coprime(a, b):
    return gcd(a, b) == 1

def mod_inverse(a, m):
    r = extended_gcd(a, m)
    if r[0] != 1:
        return sys.exit("mod_inverse: inverse does not exist (gcd(a, m) != 1)")
    else:
        return (r[1] % m + m) % m

def is_prime(n):
    if n < 2:
        return False
    if n == 2:
        return True
    if n % 2 == 0:
        return False
    i = 3
    while i * i <= n:
        if n % i == 0:
            return False
        i = i + 2
    return True

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
if not is_prime(p):
    sys.exit("p must be prime")
if not is_prime(q):
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
if not is_coprime(e, phi):
    sys.exit("e must be coprime to phi(n)")
sys.stdout.write("Verified gcd(e, phi(n)) = ")
print(gcd(e, phi))
print("")
d = mod_inverse(e, phi)
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
cipher = mod_pow(m, e, n)
sys.stdout.write("Encrypted cipher c = m^e mod n = ")
print(cipher)
decrypted = mod_pow(cipher, d, n)
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
signature = mod_pow(message_to_sign, d, n)
sys.stdout.write("Digital signature s = m^d mod n = ")
print(signature)
verified_message = mod_pow(signature, e, n)
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
