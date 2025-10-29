
extern void puts(char *str);

void print_is_even(int n) {
    if (n % 2 == 0) {
        puts("even!");
    } else {
        puts("odd!");
    }
}
