// Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
long test_let(void) {
    long a = 42;
    long b = 10;
    b = 50;
    return a + b;
}

int main(void) {
    puts("Test: let and let mut Keywords");
    long x = 10;
    puts("let x = 10");
    printf("%ld\n", x);
    long y = 5;
    puts("let mut y = 5");
    printf("%ld\n", y);
    y = 20;
    puts("After y = 20:");
    printf("%ld\n", y);
    x = 100;
    puts("After let x = 100 (shadowing):");
    printf("%ld\n", x);
    long result = x + y;
    puts("let result = x + y");
    printf("%ld\n", result);
    puts("test_let():");
    printf("%ld\n", test_let());
    return 0;
}
