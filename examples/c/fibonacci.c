#include <stdio.h>

long fib(int n) {
    long a = 0;
    long b = 1;
    int i = 0;
    while (i < n) {
        long c = a + b;
        a = b;
        b = c;
        i = i + 1;
    }
    return a;
}

int main(void) {
    int i = 0;
    while (i < 10) {
        printf("%ld\n", fib(i));
        i = i + 1;
    }
    return 0;
}
