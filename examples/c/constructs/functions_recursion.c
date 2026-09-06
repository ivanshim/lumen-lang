// Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
long factorial(long n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

long countdown(long n) {
    if (n <= 0) {
        puts("Done");
    } else {
        printf("%ld\n", n);
        return countdown(n - 1);
    }
}

int main(void) {
    puts("Test: Recursion");
    puts("Factorial of 5:");
    printf("%ld\n", factorial(5));
    puts("Countdown from 3:");
    countdown(3);
    return 0;
}
