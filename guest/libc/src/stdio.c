
#include "stdio.h"
#include "internal/stdio.h"
#include "semblance/syscall.h"
#include "stdlib.h"
#include "string.h"

struct FILE {
    int fd;
};

FILE* stderr = NULL;
FILE* stdout = NULL;

int __stdio_init() {
    stderr = fopen("/dev/stderr", "w");
    if (stderr == NULL) return 1;
    stdout = fopen("/dev/stdout", "w");
    if (stderr == NULL) return 2;
    return 0;
}

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
    size_t len = strlen(str);
    size_t written = fwrite(str, sizeof(char), len, stdout);
    if (written != len) return EOF;
    if (putchar('\n') == EOF) return EOF;
    return len + 1;
}

int putchar(int c) {
    uint8_t data = c;
    size_t written = fwrite(&data, sizeof(uint8_t), 1, stdout);
    if (written != 1) return EOF;
    return (int)data;
}

FILE *fopen(const char *path, const char *mode) {
    int fd = semblance_syscall_fopen(path, mode);
    if (fd < 0) return NULL;
    FILE* file = malloc(sizeof(FILE));
    if (file == NULL) return NULL;
    file->fd = fd;
    return file;
}

size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream) {
    return 0;
}

int fseek(FILE *stream, long int offset, int whence) {
    return 0;
}

size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream) {
    if (stream == NULL) return 0;
    size_t written = semblance_syscall_fwrite(stream->fd, ptr, size * nmemb);
    return written / size;
}

int fclose(FILE *f) {
    return -1;
}

int fflush(FILE *f) {
    return -1;
}

long ftell(FILE *f) {
    return -1;
}

int remove(const char *path) {
    return -1;
}

int rename(const char *src, const char *dst) {
    return -1;
}

int sscanf(const char *str, const char *format, ...) {
    return 0;
}
