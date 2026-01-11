
#include "stdio.h"

FILE* stderr = NULL;
FILE* stdout = NULL;

int snprintf(char *str, size_t size, const char *format, ...) {
    return 0;
}

int fprintf(FILE *f, const char *format, ...) {
    return 0;
}

int vfprintf(FILE *stream, const char *format, va_list arg) {
    return 0;
}

int vsnprintf(char *buffer, size_t size, const char *format, va_list argptr) {
    return 0;
}

int printf(const char *format, ...) {
    return 0;
}

int puts(const char *str) {
    return 0;
}

int putchar(int c) {
    return 0;
}

FILE *fopen(const char *path, const char *mode) {
    return NULL;
}

size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream) {
    return 0;
}

int fseek(FILE *stream, long int offset, int whence) {
    return 0;
}

size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream) {
    return 0;
}

int fclose(FILE *f) {
    return 0;
}

int fflush(FILE *f) {
    return 0;
}

long ftell(FILE *f) {
    return 0;
}

int remove(const char *path) {
    return 0;
}

int rename(const char *src, const char *dst) {
    return 0;
}

int sscanf(const char *str, const char *format, ...) {
    return 0;
}
