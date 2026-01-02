let SCALE = 10000000000;

let x = SCALE / 5;
let x2 = (x * x) / SCALE;

let term = x;
let sum1 = term;
let k = 1;

while term > 0 {
    term = (term * x2) / SCALE;
    k = k + 2;

    if (k / 2) * 2 == k {
        sum1 = sum1 - (term / k);
    } else {
        sum1 = sum1 + (term / k);
    }
}

let x = SCALE / 239;
let x2 = (x * x) / SCALE;

let term = x;
let sum2 = term;
let k = 1;

while term > 0 {
    term = (term * x2) / SCALE;
    k = k + 2;

    if (k / 2) * 2 == k {
        sum2 = sum2 - (term / k);
    } else {
        sum2 = sum2 + (term / k);
    }
}

let pi_scaled = (16 * sum1) - (4 * sum2);

print(pi_scaled);
