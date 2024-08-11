
#include "leb128.h"
#include <stdbool.h>
#include <stdio.h>
#include <sys/types.h>

ULeb128Decode32Result u_leb128_decode_32(u_leb128_prefixed data) {
    ULeb128Decode32Result out;
    out.value = 0;
    out.data = NULL;

    u_int32_t shift = 0;
    size_t byte_idx = 0;
    while (true) {
        u_int8_t byte = data[byte_idx];
        out.value |= (byte & ~(1 << 7)) << shift;
        if ((byte & (1 << 7)) == 0) {
            out.data = ((u_int8_t*) data) + byte_idx + 1;
            return out;
        };
        shift += 7;
        byte_idx++;
    }

    return out;
}
