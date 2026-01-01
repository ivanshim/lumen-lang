let scale = 100000;
let pi = 3 * scale;
let k = 1;
let sign = 1;
let iterations = 0;
let max_iter = 1000;

while iterations < max_iter {
    let denom = (2 * k) * (2 * k + 1) * (2 * k + 2);
    let term = (4 * scale) / denom;
    pi = pi + sign * term;
    sign = -sign;
    k = k + 1;
    iterations = iterations + 1;
}

print(pi);
