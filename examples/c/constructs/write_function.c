// Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    printf("Hello");
    printf(" ");
    printf("World");
    printf("!");
    puts("");
    printf("Numbers: ");
    long i = 1;
    while (i <= 5) {
        printf("%ld", i);
        if (i < 5) {
            printf(", ");
        }
        i = i + 1;
    }
    puts("");
    return 0;
}
