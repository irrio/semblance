
#pragma once

#include <stdint.h>
#include <stddef.h>

typedef u_int8_t u_leb128_prefixed[];

typedef struct {
    u_int32_t value;
    u_int8_t *data;
} ULeb128Decode32Result;

ULeb128Decode32Result u_leb128_decode_32(u_leb128_prefixed data);
