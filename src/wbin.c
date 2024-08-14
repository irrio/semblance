
#include "wbin.h"
#include "wmod.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

WasmDecodeResult wbin_err(WasmDecodeErrorCode error_code, int cause) {
    WasmDecodeResult out;
    out.state = WasmDecodeErr;
    out.value.error.code = error_code;
    out.value.error.cause =  cause;
    return out;
}

WasmDecodeResult wbin_err_io(int cause) {
    return wbin_err(WasmDecodeErrIo, cause);
}

WasmDecodeResult wbin_ok(void *next_data) {
    WasmDecodeResult out;
    out.state = WasmDecodeOk;
    out.value.next_data = next_data;
    return out;
}

void *wbin_decode_leb128(u_leb128_prefixed data, u_int32_t *out) {
    u_int32_t shift = 0;
    size_t byte_idx = 0;
    while (true) {
        u_int8_t byte = data[byte_idx];
        *out |= (byte & ~(1 << 7)) << shift;
        if ((byte & (1 << 7)) == 0) {
            break;
        };
        shift += 7;
        byte_idx++;
    }
    return data + byte_idx + 1;
}

WasmDecodeResult wbin_decode_val_type(void *data, WasmValueType *out) {
    u_int8_t *bytes = data;
    switch (bytes[0]) {
        case 0x7F:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumI32;
            break;
        case 0x7E:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumI64;
            break;
        case 0x7D:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumF32;
            break;
        case 0x7C:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumF64;
            break;
        case 0x7B:
            out->kind = WasmValueTypeVec;
            out->value.vec = WasmVecV128;
            break;
        case 0x70:
            out->kind = WasmValueTypeRef;
            out->value.ref = WasmRefFunc;
            break;
        case 0x6F:
            out->kind = WasmValueTypeRef;
            out->value.ref = WasmRefExtern;
            break;
        default:
            return wbin_err(WasmDecodeErrUnknownValueType, 0);
    }
    return wbin_ok(&bytes[1]);
}

WasmDecodeResult wbin_decode_result_type(void *data, WasmResultType *out) {
    u_int32_t len = 0;
    data = wbin_decode_leb128(data, &len);
    while (len > 0) {
        WasmValueType valtype;
        WasmDecodeResult valtype_result = wbin_decode_val_type(data, &valtype);
        if (!wbin_is_ok(valtype_result)) return valtype_result;
        size_t idx = wmod_result_type_push_back(out, &valtype);
        data = valtype_result.value.next_data;
        len--;
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_type(void *data, WasmFuncType *out) {
    u_int8_t *bytes = data;
    printf("decoding type %x\n", bytes[0]);
    if (*bytes != 0x60) return wbin_err(WasmDecodeErrInvalidType, 0);
    WasmDecodeResult input_result = wbin_decode_result_type(&bytes[1], &out->input_type);
    if (!wbin_is_ok(input_result)) return input_result;
    return wbin_decode_result_type(input_result.value.next_data, &out->output_type);
}

WasmDecodeResult wbin_decode_types(void *data, WasmModule *wmod) {
    u_int32_t len = 0;
    data = wbin_decode_leb128(data, &len);
    printf("num types in section: %d\n", len);

    while (len > 0) {
        WasmFuncType decoded_type;
        wmod_func_type_init(&decoded_type);
        WasmDecodeResult result = wbin_decode_type(data, &decoded_type);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_type(wmod, &decoded_type);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_funcs(void *data, WasmModule *wmod) {
    u_int32_t len = 0;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmFunc decoded_func;
        wmod_func_init(&decoded_func);
        data = wbin_decode_leb128(data, &decoded_func.type_idx);
        wmod_push_back_func(wmod, &decoded_func);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_section(WasmSectionId id, void *section, WasmModule *wmod) {
    printf("section_id: %d\n", id);
    switch (id) {
        case SectionIdType:
            return wbin_decode_types(section, wmod);
        case SectionIdFunction:
            return wbin_decode_funcs(section, wmod);
        case SectionIdCustom:
        case SectionIdImport:
        case SectionIdTable:
        case SectionIdMemory:
        case SectionIdGlobal:
        case SectionIdExport:
        case SectionIdStart:
        case SectionIdElement:
        case SectionIdCode:
        case SectionIdData:
        case SectionIdDataCount:
            return wbin_ok(section);
        default:
            return wbin_err(WasmDecodeErrUnknownSectionId, id);
    }
}

WasmDecodeResult wbin_decode_sections(off_t size, WasmSectionHeader *section, WasmModule *wmod) {
    while (size > 0) {
        u_int32_t len = 0;
        void* data = wbin_decode_leb128(section->data, &len);
        WasmDecodeResult sec_result = wbin_decode_section(section->section_id, data, wmod);
        if (!wbin_is_ok(sec_result)) return sec_result;
        size -= (len + 1);
        section = (WasmSectionHeader*) data + len;
    }
    return wbin_ok(section);
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
    switch (result.value.error.code) {
        case WasmDecodeErrIo:
            return "unable to open file";
        case WasmDecodeErrMagicBytes:
            return "not a wasm module";
        case WasmDecodeErrUnsupportedVersion:
            return "unsupported version";
        case WasmDecodeErrOom:
            return "out of memory";
        case WasmDecodeErrUnknownSectionId:
            return "unknown section id";
        case WasmDecodeErrInvalidType:
            return "invalid type";
        case WasmDecodeErrUnknownValueType:
            return "unknown value type";
        default:
            return "unknown state";
    }
}

char *wbin_explain_error_cause(WasmDecodeResult result) {
    switch (result.value.error.code) {
        case WasmDecodeErrIo:
            return strerror(result.value.error.cause);
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
