
#include "leb128.h"
#include <stdio.h>
#include <sys/types.h>

ULeb128Decode32Result u_leb128_decode_32(size_t len, u_leb128_prefixed data) {
    ULeb128Decode32Result out;
    out.value = 0;
    out.data = NULL;

    u_int32_t shift = 0;
    for (size_t i = 0; i < len; i++) {
        u_int8_t byte = data[i];
        out.value |= (byte & ~(1 << 7)) << shift;
        if ((byte & (1 << 7)) == 0) {
            out.data = ((u_int8_t*) data) + i + 1;
            return out;
        };
        shift += 7;
    }

    return out;
}
