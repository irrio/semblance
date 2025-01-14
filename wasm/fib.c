
int fib(int n) {
    if (n <= 2) return 2;
    return fib(n - 1) + fib(n - 2);
}

int run() {
    return fib(5);
}
