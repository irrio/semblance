
#include "string.h"
#include "ctype.h"
#include "stdlib.h"

void *memset(void *str, int c, size_t n) {
    return __builtin_memset(str, c, n);
}

void *memcpy(void *dst, const void *src, size_t len) {
    return __builtin_memcpy(dst, src, len);
}

void *memmove(void *dst, const void *src, size_t len) {
    return __builtin_memcpy(dst, src, len);
}

size_t strlen(const char *str) {
    size_t i = 0;
    while (str[i] != 0) {
        i++;
    }
    return i;
}

char *strncpy(char *dst, const char* src, size_t n) {
    return __builtin_memcpy(dst, src, n);
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 != '\0' && *s1 == *s2) {
        s1++;
        s2++;
    }
    return *(uint8_t*)s1 - *(uint8_t*)s2;
}

int strncmp(const char *s1, const char *s2, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (s1[i] == '\0' || s1[i] != s2[i]) break;
    }
    return ((uint8_t*)s1)[i] - ((uint8_t*)s2)[i];
}

int strcasecmp(const char *s1, const char *s2) {
    while (*s1 != '\0' && toupper(*s1) == toupper(*s2)) {
        s1++;
        s2++;
    }
    return toupper(*(uint8_t*)s1) - toupper(*(uint8_t*)s2);
}

int strncasecmp(const char *s1, const char *s2, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (s1[i] == '\0' || toupper(s1[i]) != toupper(s2[i])) break;
    }
    return ((uint8_t*)s1)[i] - ((uint8_t*)s2)[i];
}

char *strrchr(const char *s, int c) {
    size_t len = strlen(s);
    const char *ptr = &s[len];
    while (ptr >= s) {
        if (*ptr == c) return (char*)ptr;
        s--;
    }
    return NULL;
}

char *strchr(const char *s, int c) {
    while (*s != '\0') {
        if (*s == c) return (char*)s;
    }
    return c == '\0' ? (char*)s : NULL;
}

static size_t str_overlap(const char *haystack, const char *needle, size_t n) {
    for (int i = 0; i < n; i++) {
        if (haystack[i] != needle[i]) return i;
    }
    return n;
}

char *strstr(const char *haystack, const char *needle) {
    size_t haystack_len = strlen(haystack);
    size_t needle_len = strlen(needle);
    if (needle_len == 0) return (char*)haystack;
    if (needle_len > haystack_len) return NULL;
    size_t stop = haystack_len - needle_len;
    for (size_t start = 0; start <= stop; start++) {
        if (str_overlap(&haystack[start], needle, needle_len) == needle_len)
            return (char*)&haystack[start];
    }
    return NULL;
}

char *strdup(const char *s) {
    size_t len = strlen(s);
    char *dup = malloc(len + 1);
    if (dup == NULL) return NULL;
    return __builtin_memcpy(dup, s, len + 1);
}
