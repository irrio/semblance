
#pragma once

#include <stddef.h>
#include <stdbool.h>

typedef struct {
    size_t len;
    size_t cap;
    void *ptr;
} Vec;

void vec_init(Vec *vec);
void vec_init_with_capacity(Vec *vec, size_t with, size_t capacity);
size_t vec_push_back(Vec *vec, size_t width, void *data);
bool vec_pop_back(Vec *vec, size_t width, void *out);

#define VEC(ty) Vec
