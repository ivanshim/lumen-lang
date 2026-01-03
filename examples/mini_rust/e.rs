let SCALE = 10000000000;
let sum = SCALE;
let term = SCALE;
let n = 1;

while term > 0 {
    term = term / n;
    sum = sum + term;
    n = n + 1;
}

let int_part = sum / SCALE;
let frac_part = sum % SCALE;

print(int_part);
print(frac_part);
