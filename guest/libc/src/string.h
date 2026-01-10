
#pragma once

#include "inttypes.h"

void *memset(void *str, int c, size_t n);
void *memcpy(void *dst, const void *src, size_t len);
void *memmove(void *dst, const void *src, size_t len);

size_t strlen(const char *s);
char *strncpy(char *dest, const char *src, size_t n);
int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
int strcasecmp(const char *s1, const char *s2);
int strncasecmp(const char *s1, const char *s2, size_t len);
char *strrchr(const char *s, int c);
char *strdup(const char *s);
char *strchr(const char *s, int c);
char *strstr(const char *haystack, const char *needle);

#define NULL ((void*)0)
