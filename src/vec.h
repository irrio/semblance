
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
void vec_init_with_zeros(Vec *vec, size_t width, size_t size);
void vec_init_with_size(Vec *vec, size_t width, size_t size, void *initval);
size_t vec_push_back(Vec *vec, size_t width, void *data);
bool vec_pop_back(Vec *vec, size_t width, void *out);
bool vec_pop_back_and_drop(Vec *vec);
void *vec_at(Vec *vec, size_t width, size_t idx);
void vec_clone(Vec *src, Vec *dst, size_t width);
void vec_free(Vec *vec);

#define VEC(ty) Vec
