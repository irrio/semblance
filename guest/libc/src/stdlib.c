
#include "stdlib.h"
#include "semblance/syscall.h"

void exit(int status) {
    semblance_syscall_exit(status);
}

int system(const char *command) {
    return 1;
}

int atoi(const char *str) {
    return semblance_syscall_parse_i32(str);
}

double atof(const char *str) {
    return semblance_syscall_parse_f64(str);
}

int abs(int num) {
    return __builtin_abs(num);
}
