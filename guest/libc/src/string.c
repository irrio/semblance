
#include "string.h"

void *memset(void *str, int c, size_t n) {
    __builtin_memset(str, c, n);
    return str;
}

void *memcpy(void *dst, const void *src, size_t len) {
    __builtin_memcpy(dst, src, len);
    return dst;
}

void *memmove(void *dst, const void *src, size_t len) {
    __builtin_memcpy(dst, src, len);
    return dst;
}

size_t strlen(const char *str) {
    size_t i = 0;
    while (str[i] != 0) {
        i++;
    }
    return i;
}

char *strncpy(char *dst, const char* src, size_t n) {
    memcpy(dst, src, n);
    return dst;
}

int strcmp(const char *s1, const char *s2) {
    return 0;
}

int strncmp(const char *s1, const char *s2, size_t n) {
    return 0;
}

int strcasecmp(const char *s1, const char *s2) {
    return 0;
}

int strncasecmp(const char *s1, const char *s2, size_t len) {
    return 0;
}

char *strrchr(const char *s, int c) {
    return NULL;
}

char *strdup(const char *s) {
    return NULL;
}

char *strchr(const char *s, int c) {
    return NULL;
}

char *strstr(const char *haystack, const char *needle) {
    return NULL;
}
