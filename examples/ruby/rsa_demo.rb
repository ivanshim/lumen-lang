# Ported from examples/lumen/rsa_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def mod_pow(base, exp, m)
    result = 1
    base = base % m
    while exp > 0 do
        if exp % 2 == 1 then
            result = (result * base) % m
        end
        exp = exp / 2
        base = (base * base) % m
    end
    return result
end

def gcd(a, b)
    while b != 0 do
        t = b
        b = a % b
        a = t
    end
    return a
end

def extended_gcd(a, b)
    if b == 0 then
        return [a, 1, 0]
    else
        r = extended_gcd(b, a % b)
        g = r[0]
        x = r[2]
        y = r[1] - (a / b) * r[2]
        return [g, x, y]
    end
end

def is_coprime(a, b)
    return gcd(a, b) == 1
end

def mod_inverse(a, m)
    r = extended_gcd(a, m)
    if r[0] != 1 then
        return raise("mod_inverse: inverse does not exist (gcd(a, m) != 1)")
    else
        return (r[1] % m + m) % m
    end
end

def is_prime(n)
    if n < 2 then
        return false
    end
    if n == 2 then
        return true
    end
    if n % 2 == 0 then
        return false
    end
    i = 3
    while i * i <= n do
        if n % i == 0 then
            return false
        end
        i = i + 2
    end
    return true
end

puts("=== RSA Cryptography Demonstration ===")
puts("")
puts("Step 1: Key Generation")
puts("----------------------")
p = 61
q = 53
print("Selected prime p = ")
puts(p)
print("Selected prime q = ")
puts(q)
if !is_prime(p) then
    raise("p must be prime")
end
if !is_prime(q) then
    raise("q must be prime")
end
if p == q then
    raise("p and q must be distinct")
end
puts("")
n = p * q
print("Computed n = p * q = ")
puts(n)
puts("")
phi = (p - 1) * (q - 1)
print("Computed phi(n) = (p-1)(q-1) = ")
puts(phi)
puts("")
e = 17
print("Selected public exponent e = ")
puts(e)
if e <= 1 || e >= phi then
    raise("e must satisfy 1 < e < phi(n)")
end
if !is_coprime(e, phi) then
    raise("e must be coprime to phi(n)")
end
print("Verified gcd(e, phi(n)) = ")
puts(gcd(e, phi))
puts("")
d = mod_inverse(e, phi)
print("Computed private exponent d = ")
puts(d)
verification = (d * e) % phi
print("Verification: d * e mod phi(n) = ")
puts(verification)
if verification != 1 then
    raise("Private key computation failed: d * e mod phi(n) must equal 1")
end
puts("")
puts("=== Generated Keys ===")
print("Public key:  (e, n) = (")
print(e)
print(", ")
print(n)
puts(")")
print("Private key: (d, n) = (")
print(d)
print(", ")
print(n)
puts(")")
puts("")
puts("=== Encryption and Decryption ===")
puts("")
m = 42
print("Original message m = ")
puts(m)
if m < 0 || m >= n then
    raise("Message must be in range 0 to n-1")
end
cipher = mod_pow(m, e, n)
print("Encrypted cipher c = m^e mod n = ")
puts(cipher)
decrypted = mod_pow(cipher, d, n)
print("Decrypted message m' = c^d mod n = ")
puts(decrypted)
if decrypted == m then
    puts("SUCCESS: Decryption successful: m' = m")
else
    raise("Decryption failed")
end
puts("")
puts("=== Digital Signature ===")
puts("")
message_to_sign = 100
print("Message to sign = ")
puts(message_to_sign)
if message_to_sign < 0 || message_to_sign >= n then
    raise("Message to sign must be in range 0 to n-1")
end
signature = mod_pow(message_to_sign, d, n)
print("Digital signature s = m^d mod n = ")
puts(signature)
verified_message = mod_pow(signature, e, n)
print("Verified message m' = s^e mod n = ")
puts(verified_message)
if verified_message == message_to_sign then
    puts("SUCCESS: Signature verification successful")
else
    raise("Signature verification failed")
end
puts("")
puts("=== Summary ===")
puts("")
puts("RSA Properties Demonstrated:")
puts("1. Key Generation: Generated (e,n) and (d,n) from primes p and q")
puts("2. Encryption: c = m^e mod n")
puts("3. Decryption: m = c^d mod n")
puts("4. Digital Signature: s = m^d mod n")
puts("5. Signature Verification: m = s^e mod n")
puts("")
puts("Mathematical Correctness:")
puts("  (m^e)^d == m^(ed) == m^1 == m (mod n)")
puts("  because ed == 1 (mod phi(n)) by construction")
puts("")
puts("Security Note: This demo uses 6-bit primes. Real RSA requires")
puts("              at least 2048-bit primes for security.")
