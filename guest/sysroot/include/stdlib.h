
#pragma once

#include "inttypes.h"

#define NULL ((void*)0)

int atoi(const char *str);
double atof(const char *str);
int abs(int num);

void *malloc(int size);
void *realloc(void *ptr, size_t size);
void* calloc(size_t num, size_t size);
void free(void *mem);

void exit(int status);
int system(const char *command);
