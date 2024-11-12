
#pragma once

#include <stddef.h>
#include <stdbool.h>

typedef struct {
    size_t len;
    size_t cap;
    void *ptr;
} Vec;

void vec_init(Vec *vec);
void vec_init_with_capacity(Vec *vec, size_t width, size_t capacity);
void vec_init_with_size(Vec *vec, size_t width, size_t size, void *initval);
size_t vec_push_back(Vec *vec, size_t width, void *data);
bool vec_pop_back(Vec *vec, size_t width, void *out);

#define VEC(ty) Vec
