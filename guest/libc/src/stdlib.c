
#include "stdlib.h"

int atoi(const char *str) {
    return 0;
}

double atof(const char *str) {
    return 0;
}

int abs(int num) {
    return num;
}

void *malloc(int size) {
    return NULL;
}

void *realloc(void *ptr, size_t size) {
    return NULL;
}

void *calloc(size_t num, size_t size) {
    return NULL;
}

void free(void *mem) {
    return;
}

void exit(int status) {
    __builtin_unreachable();
}

int system(const char *command) {
    return 1;
}
