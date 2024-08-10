
#include "wmod.h"
#include "leb128.h"
#include <fcntl.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>

const int WMOD_ERR_IO                   = 1;
const int WMOD_ERR_MAGIC_BYTES          = 2;
const int WMOD_ERR_UNSUPPORTED_VERSION  = 3;
const int WMOD_ERR_OOM                  = 4;

const u_int8_t WMOD_SECTION_ID_CUSTOM       = 0;
const u_int8_t WMOD_SECTION_ID_TYPE         = 1;
const u_int8_t WMOD_SECTION_ID_IMPORT       = 2;
const u_int8_t WMOD_SECTION_ID_FUNCTION     = 3;
const u_int8_t WMOD_SECTION_ID_TABLE        = 4;
const u_int8_t WMOD_SECTION_ID_MEMORY       = 5;
const u_int8_t WMOD_SECTION_ID_GLOBAL       = 6;
const u_int8_t WMOD_SECTION_ID_EXPORT       = 7;
const u_int8_t WMOD_SECTION_ID_START        = 8;
const u_int8_t WMOD_SECTION_ID_ELEMENT      = 9;
const u_int8_t WMOD_SECTION_ID_CODE         = 10;
const u_int8_t WMOD_SECTION_ID_DATA         = 11;
const u_int8_t WMOD_SECTION_ID_DATA_COUNT   = 12;

WasmModule *wmod_mk_err(WmodErr *err, int error_code, int cause) {
    err->wmod_err = error_code;
    err->cause = cause;
    return (WasmModule*) -1;
}

WasmModule *wmod_err_ok(WasmModule *wmod, WmodErr *err) {
    err->wmod_err = 0;
    err->cause = 0;
    return wmod;
}

size_t wmod_count_sections(off_t size, WasmSectionHeader *section) {
    size_t count = 0;
    while (size > 0) {
        ULeb128Decode32Result decoded = u_leb128_decode_32(size, section->data);
        if (decoded.data == NULL) break;
        size -= (decoded.value + 1);
        section = (WasmSectionHeader*) decoded.data + decoded.value;
        count++;
    }
    return count;
}

WasmModule *wmod_validate(off_t size, WasmHeader *header, WmodErr *err) {
    if (size < sizeof(WasmHeader)) return wmod_mk_err(err, WMOD_ERR_MAGIC_BYTES, 0);

    if (
        header->magic_bytes[0] != '\0'
        || header->magic_bytes[1] != 'a'
        || header->magic_bytes[2] != 's'
        || header->magic_bytes[3] != 'm'
    ) return wmod_mk_err(err, WMOD_ERR_MAGIC_BYTES, 0);

    if (header->version != 1) return wmod_mk_err(err, WMOD_ERR_UNSUPPORTED_VERSION, 0);

    size_t num_sections = wmod_count_sections(size - sizeof(WasmHeader), header->sections);
    WasmModule* out = malloc(sizeof(WasmModule) + (sizeof(WasmSection) * num_sections));

    if (out == NULL) return wmod_mk_err(err, WMOD_ERR_OOM, 0);

    out->total_size = size;
    out->num_sections = num_sections;
    out->raw_data = header;

    return wmod_err_ok(out, err);
}

WasmModule *wmod_read(char *path, WmodErr *err) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return wmod_mk_err(err, WMOD_ERR_IO, errno);

    struct stat stats;
    int stat_err = fstat(fd, &stats);
    if (stat_err == -1) return wmod_mk_err(err, WMOD_ERR_IO, errno);

    WasmHeader* data = mmap(NULL, stats.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if ((size_t) data == -1) return wmod_mk_err(err, WMOD_ERR_IO, errno);

    int close_err = close(fd);
    if (close_err != 0) return wmod_mk_err(err, WMOD_ERR_IO, errno);

    return wmod_validate(stats.st_size, data, err);
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
        case WMOD_ERR_OOM:
            return "out of memory";
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
