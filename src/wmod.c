
#include "wmod.h"
#include <fcntl.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>

const int WMOD_ERR_IO = 1;
const int WMOD_ERR_MAGIC_BYTES = 2;
const int WMOD_ERR_UNSUPPORTED_VERSION = 3;

WmodErr wmod_mk_err(int wmod_err, int cause) {
    WmodErr out;
    out.wmod_err = wmod_err;
    out.cause = cause;
    return out;
}

WmodErr wmod_err_ok() {
    WmodErr out;
    out.wmod_err = 0;
    out.cause = 0;
    return out;
}

WmodErr wmod_validate(off_t size, WasmHeader *header) {
    if (size < sizeof(WasmHeader)) return wmod_mk_err(WMOD_ERR_MAGIC_BYTES, 0);

    if (
        header->magic_bytes[0] != '\0'
        || header->magic_bytes[1] != 'a'
        || header->magic_bytes[2] != 's'
        || header->magic_bytes[3] != 'm'
    ) return wmod_mk_err(WMOD_ERR_MAGIC_BYTES, 0);

    if (header->version != 1) return wmod_mk_err(WMOD_ERR_UNSUPPORTED_VERSION, 0);

    return wmod_err_ok();
}

WmodErr wmod_read(WasmModule *wmod, char *path) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return wmod_mk_err(WMOD_ERR_IO, errno);

    struct stat stats;
    int stat_err = fstat(fd, &stats);
    if (stat_err == -1) return wmod_mk_err(WMOD_ERR_IO, errno);
    wmod->size = stats.st_size;

    WasmHeader* data = mmap(NULL, stats.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if ((size_t) data == -1) return wmod_mk_err(WMOD_ERR_IO, errno);
    wmod->data = data;

    int close_err = close(fd);
    if (close_err != 0) return wmod_mk_err(WMOD_ERR_IO, errno);

    return wmod_validate(stats.st_size, data);
}

int wmod_failed(WmodErr err) {
    return err.wmod_err != 0;
}

char *wmod_str_error(WmodErr err) {
    switch (err.wmod_err) {
        case WMOD_ERR_IO:
            return "unable to open file";
        case WMOD_ERR_MAGIC_BYTES:
            return "not a wasm module";
        case WMOD_ERR_UNSUPPORTED_VERSION:
            return "unsupported version";
        default:
            return "unknown error";
    }
}

char *wmod_str_error_cause(WmodErr err) {
    switch (err.wmod_err) {
        case WMOD_ERR_IO:
            return strerror(err.cause);
        case 0:
            return "";
        default:
            if (err.cause == 0) {
                return "";
            } else {
                return "unknown cause";
            }
    }
}
