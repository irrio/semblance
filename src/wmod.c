
#include "wmod.h"

const int WMOD_ERR_NOT_FOUND = 1;

int wmod_read(WasmModule *wmod, char *path) {
    return WMOD_ERR_NOT_FOUND;
}

char *wmod_str_error(int err) {
    switch (err) {
        case WMOD_ERR_NOT_FOUND:
            return "file not found";
        default:
            return "unknown error";
    }
}
