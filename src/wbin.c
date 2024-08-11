
#include "wbin.h"
#include "leb128.h"
#include "wmod.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

WasmDecodeResult wbin_err(WasmDecodeState state, int cause) {
    WasmDecodeResult out;
    out.state = state;
    out.cause = cause;
    return out;
}

WasmDecodeResult wbin_err_io(int cause) {
    return wbin_err(WasmDecodeErrIo, cause);
}

WasmDecodeResult wbin_ok() {
    return wbin_err(WasmDecodeOk, 0);
}

WasmDecodeResult wbin_decode_types(u_leb128_prefixed section, WasmModule *wmod) {
    ULeb128Decode32Result decoded = u_leb128_decode_32(section);
    printf("num types in section: %d\n", decoded.value);
    return wbin_ok();
}

WasmDecodeResult wbin_decode_section(WasmSectionId id, void *section, WasmModule *wmod) {
    printf("section_id: %d\n", id);
    switch (id) {
        case SectionIdType:
            return wbin_decode_types(section, wmod);
        case SectionIdCustom:
        case SectionIdImport:
        case SectionIdFunction:
        case SectionIdTable:
        case SectionIdMemory:
        case SectionIdGlobal:
        case SectionIdExport:
        case SectionIdStart:
        case SectionIdElement:
        case SectionIdCode:
        case SectionIdData:
        case SectionIdDataCount:
            return wbin_ok();
        default:
            return wbin_err(WasmDecodeErrUnknownSectionId, id);
    }
}

WasmDecodeResult wbin_decode_sections(off_t size, WasmSectionHeader *section, WasmModule *wmod) {
    while (size > 0) {
        ULeb128Decode32Result decoded = u_leb128_decode_32(section->data);
        if (decoded.data == NULL) return wbin_err(WasmDecodeErrLeb128, 0);
        WasmDecodeResult sec_result = wbin_decode_section(section->section_id, decoded.data, wmod);
        if (!wbin_is_ok(sec_result)) return sec_result;
        size -= (decoded.value + 1);
        section = (WasmSectionHeader*) decoded.data + decoded.value;
    }
    return wbin_ok();
}

WasmDecodeResult wbin_read_module(char *path, WasmModule *wmod) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return wbin_err_io(errno);

    struct stat stats;
    int stat_err = fstat(fd, &stats);
    if (stat_err == -1) return wbin_err_io(errno);

    WasmHeader* data = mmap(NULL, stats.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if ((size_t) data == -1) return wbin_err_io(errno);

    int close_err = close(fd);
    if (close_err != 0) return wbin_err_io(errno);

    return wbin_decode_module(stats.st_size, data, wmod);
}

WasmDecodeResult wbin_decode_module(size_t size, WasmHeader *header, WasmModule *wmod) {
    if (
        size < sizeof(WasmHeader)
        || header->magic_bytes[0] != '\0'
        || header->magic_bytes[1] != 'a'
        || header->magic_bytes[2] != 's'
        || header->magic_bytes[3] != 'm'
    ) return wbin_err(WasmDecodeErrMagicBytes, 0);

    wmod->meta.version = header->version;
    if (header->version != 1) return wbin_err(WasmDecodeErrUnsupportedVersion, 0);

    return wbin_decode_sections(size - sizeof(WasmHeader), header->sections, wmod);
}

char *wbin_explain_error_code(WasmDecodeResult result) {
    switch (result.state) {
        case WasmDecodeOk:
            return "ok";
        case WasmDecodeErrIo:
            return "unable to open file";
        case WasmDecodeErrMagicBytes:
            return "not a wasm module";
        case WasmDecodeErrUnsupportedVersion:
            return "unsupported version";
        case WasmDecodeErrOom:
            return "out of memory";
        case WasmDecodeErrLeb128:
            return "invalid numeric";
        case WasmDecodeErrUnknownSectionId:
            return "unknown section id";
        default:
            return "unknown state";
    }
}

char *wbin_explain_error_cause(WasmDecodeResult result) {
    switch (result.state) {
        case WasmDecodeErrIo:
            return strerror(result.cause);
        default:
            return "";
    }
}

bool wbin_is_ok(WasmDecodeResult result) {
    return result.state == WasmDecodeOk;
}

bool wbin_error_has_cause(WasmDecodeResult result) {
    return result.state == WasmDecodeErrIo;
}
