// Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
long double_(long x) {
    return x * 2;
}

long add_one(long x) {
    return x + 1;
}

long square(long x) {
    return x * x;
}

long multiply(long a, long b) {
    return a * b;
}

int main(void) {
    puts("Test: Pipe Operator");
    puts("Without pipe: square(add_one(double(5)))");
    printf("%ld\n", square(add_one(double_(5))));
    puts("With pipe: 5 |> double() |> add_one() |> square()");
    long result = square(add_one(double_(5)));
    printf("%ld\n", result);
    puts("10 |> double():");
    printf("%ld\n", double_(10));
    puts("3 |> double():");
    long x = double_(3);
    printf("%ld\n", multiply(x, 2));
    return 0;
}
