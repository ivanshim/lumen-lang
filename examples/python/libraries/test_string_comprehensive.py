import sys
# Ported from examples/lumen/libraries/test_string_comprehensive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def character_to_value(c):
    if is_digit(c):
        return ord(c) - ord("0")
    code = ord(c)
    if code >= ord("A") and code <= ord("Z"):
        return code - ord("A") + 10
    if code >= ord("a") and code <= ord("z"):
        return code - ord("a") + 10
    return -1

def digits_to_base_value(s, i, base):
    start = i
    value = 0
    scale = 1
    while i < len(s):
        c = s[i]
        d = character_to_value(c)
        if d < 0 or d >= base:
            break
        value = value * base + d
        scale = scale * base
        i = i + 1
    if i == start:
        sys.exit("expected digit")
    return [value, scale, i]

def numeric_literal_to_value(s, i):
    start = i
    base_prefix = 0
    while i < len(s):
        c = s[i]
        if not is_digit(c):
            break
        base_prefix = base_prefix * 10 + (ord(c) - ord("0"))
        i = i + 1
    if i == start:
        return [0, start]
    base = 10
    value = base_prefix
    if i < len(s) and s[i] == "@":
        base = base_prefix
        if base < 2 or base > 36:
            sys.exit("invalid base")
        i = i + 1
        r = digits_to_base_value(s, i, base)
        value = r[0]
        i = r[2]
    if i < len(s) and s[i] == ".":
        i = i + 1
        r2 = digits_to_base_value(s, i, base)
        frac_val = r2[0]
        frac_scale = r2[1]
        i = r2[2]
        if frac_scale > 1:
            value = value + frac_val / frac_scale
    return [value, i]

def string_to_value(s):
    if len(s) == 0:
        return 0
    i = 0
    r = numeric_literal_to_value(s, i)
    num = r[0]
    i = r[1]
    if i < len(s) and s[i] == "/":
        i = i + 1
        if i == len(s):
            return s
        r2 = numeric_literal_to_value(s, i)
        denom = r2[0]
        i = r2[1]
        if i != len(s):
            return s
        return num / denom
    if i == len(s):
        return num
    return s

def is_digit(c):
    o = ord(c)
    return o >= ord("0") and o <= ord("9")

def is_alpha(c):
    o = ord(c)
    return (o >= ord("A") and o <= ord("Z")) or (o >= ord("a") and o <= ord("z"))

def is_alnum(c):
    return is_alpha(c) or is_digit(c)

def char_to_upper(c):
    o = ord(c)
    if o >= ord("a") and o <= ord("z"):
        return chr(o - 32)
    else:
        return c

def char_to_lower(c):
    o = ord(c)
    if o >= ord("A") and o <= ord("Z"):
        return chr(o + 32)
    else:
        return c

def string_to_lower(s):
    result = ""
    i = 0
    while i < len(s):
        result = result + char_to_lower(s[i])
        i = i + 1
    return result

print("=== Text Processing Pipeline ===")
print("")
print("Password Strength Validator:")
def has_digit_char(s):
    digit_i = 0
    while digit_i < len(s):
        if is_digit(s[digit_i]):
            return True
        digit_i = digit_i + 1
    return False

def has_alpha_char(s):
    alpha_i = 0
    while alpha_i < len(s):
        if is_alpha(s[alpha_i]):
            return True
        alpha_i = alpha_i + 1
    return False

def has_upper_char(s):
    upper_i = 0
    c = ""
    o = 0
    while upper_i < len(s):
        c = s[upper_i]
        o = ord(c)
        if o >= ord("A") and o <= ord("Z"):
            return True
        upper_i = upper_i + 1
    return False

def has_lower_char(s):
    lower_i = 0
    c = ""
    o = 0
    while lower_i < len(s):
        c = s[lower_i]
        o = ord(c)
        if o >= ord("a") and o <= ord("z"):
            return True
        lower_i = lower_i + 1
    return False

def validate_password(pwd):
    print("  Password: '" + pwd + "'")
    if len(pwd) < 8:
        print("    WEAK: Too short (minimum 8 characters)")
        return False
    if not has_digit_char(pwd):
        print("    WEAK: Must contain at least one digit")
        return False
    if not has_alpha_char(pwd):
        print("    WEAK: Must contain at least one letter")
        return False
    if not has_upper_char(pwd):
        print("    WEAK: Must contain at least one uppercase letter")
        return False
    if not has_lower_char(pwd):
        print("    WEAK: Must contain at least one lowercase letter")
        return False
    print("    STRONG: All requirements met")
    return True

passwords = ["abc123", "PASSWORD123", "Pass123", "MyP@ssw0rd"]
pwd_i = 0
while pwd_i < len(passwords):
    validate_password(passwords[pwd_i])
    pwd_i = pwd_i + 1
print("")
print("Extract and Sum Numbers from Text:")
def extract_all_numbers(s):
    numbers = []
    current_num = ""
    extract_i = 0
    c = ""
    while extract_i < len(s):
        c = s[extract_i]
        if is_digit(c):
            current_num = current_num + c
        elif len(current_num) > 0:
            numbers.append(string_to_value(current_num))
            current_num = ""
        extract_i = extract_i + 1
    if len(current_num) > 0:
        numbers.append(string_to_value(current_num))
    return numbers

def sum_numbers_in_text(s):
    nums = extract_all_numbers(s)
    sum = 0
    sum_i = 0
    while sum_i < len(nums):
        sum = sum + nums[sum_i]
        sum_i = sum_i + 1
    return sum

texts = ["I have 5 apples and 3 oranges", "Order #123 total: 45 items", "No numbers here", "Year 2025 day 17"]
text_i = 0
text = ""
sum = 0
while text_i < len(texts):
    text = texts[text_i]
    sum = sum_numbers_in_text(text)
    print("  '" + text + "'")
    print("    Sum: " + str(sum))
    text_i = text_i + 1
print("")
print("Normalize and Compare Strings:")
def normalize_string(s):
    lower = string_to_lower(s)
    result = ""
    normalize_i = 0
    c = ""
    while normalize_i < len(lower):
        c = lower[normalize_i]
        if is_alnum(c):
            result = result + c
        normalize_i = normalize_i + 1
    return result

def strings_match_normalized(a, b):
    return normalize_string(a) == normalize_string(b)

pairs = [["Hello World", "hello world"], ["Test-123", "TEST123"], ["Lumen Lang", "LumenLang"], ["Different", "Strings"]]
pair_i = 0
a = ""
b = ""
match = False
while pair_i < len(pairs):
    a = pairs[pair_i][0]
    b = pairs[pair_i][1]
    match = strings_match_normalized(a, b)
    print("  '" + a + "' vs '" + b + "': " + str(match))
    pair_i = pair_i + 1
print("")
print("Generate Acronym from Phrase:")
def generate_acronym(phrase):
    words = []
    current_word = ""
    acronym_i = 0
    c = ""
    while acronym_i < len(phrase):
        c = phrase[acronym_i]
        if c == " ":
            if len(current_word) > 0:
                words.append(current_word)
                current_word = ""
        else:
            current_word = current_word + c
        acronym_i = acronym_i + 1
    if len(current_word) > 0:
        words.append(current_word)
    acronym = ""
    acronym_j = 0
    word = ""
    while acronym_j < len(words):
        word = words[acronym_j]
        if len(word) > 0:
            acronym = acronym + char_to_upper(word[0])
        acronym_j = acronym_j + 1
    return acronym

phrases = ["As Soon As Possible", "Frequently Asked Questions", "Light Amplification by Stimulated Emission of Radiation", "You Only Live Once"]
phrase_i = 0
phrase = ""
acronym = ""
while phrase_i < len(phrases):
    phrase = phrases[phrase_i]
    acronym = generate_acronym(phrase)
    print("  '" + phrase + "' -> " + acronym)
    phrase_i = phrase_i + 1
print("")
print("Simple Caesar Cipher (letters only, shift by 3):")
def caesar_shift_char(c):
    if not is_alpha(c):
        return c
    o = ord(c)
    if o >= ord("a") and o <= ord("z"):
        shifted = ((o - ord("a") + 3) % 26) + ord("a")
        return chr(shifted)
    else:
        shifted = ((o - ord("A") + 3) % 26) + ord("A")
        return chr(shifted)

def caesar_encrypt(s):
    result = ""
    caesar_i = 0
    c = ""
    o = 0
    shifted = 0
    while caesar_i < len(s):
        c = s[caesar_i]
        if not is_alpha(c):
            result = result + c
        else:
            o = ord(c)
            if o >= ord("a") and o <= ord("z"):
                shifted = ((o - ord("a") + 3) % 26) + ord("a")
                result = result + chr(shifted)
            else:
                shifted = ((o - ord("A") + 3) % 26) + ord("A")
                result = result + chr(shifted)
        caesar_i = caesar_i + 1
    return result

messages = ["hello world", "LUMEN LANG", "Test 123"]
msg_i = 0
msg = ""
encrypted = ""
while msg_i < len(messages):
    msg = messages[msg_i]
    encrypted = caesar_encrypt(msg)
    print("  '" + msg + "' -> '" + encrypted + "'")
    msg_i = msg_i + 1
