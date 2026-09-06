// Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long i = 0;
    while (i < 11) {
        if (i == 5) {
            i = i + 1;
            continue;
        }
        printf("%ld", i);
        if (i < 10) {
            printf(", ");
        }
        i = i + 1;
    }
    puts("");
    return 0;
}
