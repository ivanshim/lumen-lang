let pi = 3;
let k = 1;

while k < 50 {
    let numerator = 4;
    let denominator = (2 * k) * (2 * k + 1) * (2 * k + 2);

    if k % 2 == 1 {
        pi = pi + (numerator / denominator);
    } else {
        pi = pi - (numerator / denominator);
    }

    k = k + 1;
}

print(pi);
