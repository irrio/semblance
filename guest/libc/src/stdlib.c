
#include "stdlib.h"
#include "semblance/syscall.h"

void exit(int status) {
    semblance_syscall_exit(status);
}

int system(const char *command) {
    return 1;
}

int atoi(const char *str) {
    return 0;
}

double atof(const char *str) {
    return 0;
}

int abs(int num) {
    return num;
}
