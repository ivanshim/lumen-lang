// Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
long add_numbers(long a, long b) {
    return a + b;
}

int main(void) {
    puts("Test: Comments Support");
    long x = 42;
    printf("%ld\n", x);
    long result = x * 2;
    printf("%ld\n", result);
    long value = add_numbers(10, 20);
    printf("%ld\n", value);
    if (value > 20) {
        puts("Value is greater than 20");
    } else {
        puts("Value is 20 or less");
    }
    long counter = 0;
    while (counter < 3) {
        printf("%ld\n", counter);
        counter = counter + 1;
    }
    puts("Done testing comments");
    return 0;
}
