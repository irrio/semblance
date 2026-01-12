
#pragma once

#include "inttypes.h"
#include "stdarg.h"

typedef struct FILE FILE;

int snprintf(char *str, size_t size, const char *format, ...);
int fprintf(FILE *f, const char *format, ...);
int vfprintf(FILE *stream, const char *format, va_list arg);
int vsnprintf(char *buffer, size_t size, const char *format, va_list argptr);
int printf(const char *format, ...);
int puts(const char *str);
int putchar(int c);

extern FILE *stdout;
extern FILE *stderr;

FILE *fopen(const char *path, const char *mode);
size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream);
int fseek(FILE *stream, long int offset, int whence);
size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream);
int fclose(FILE *f);
int fflush(FILE *f);
long ftell(FILE *f);
int remove(const char *path);
int rename(const char *src, const char *dst);

int sscanf(const char *str, const char *format, ...);

#define NULL ((void*)0)

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
