let scale = 1000000;
let sum = scale;
let term = scale;
let n = 1;
let iterations = 0;
let max_iter = 100;

while iterations < max_iter {
    term = term / n;
    sum = sum + term;
    n = n + 1;
    iterations = iterations + 1;
}

print(sum);
