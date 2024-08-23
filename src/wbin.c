
#include "wbin.h"
#include "wmod.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/_types/_u_int32_t.h>
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
    *out = 0;
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

WasmDecodeResult wbin_decode_reftype(void *data, WasmRefType *out) {
    u_int8_t *bytes = data;
    switch (bytes[0]) {
        case 0x70:
            *out = WasmRefFunc;
            break;
        case 0x6F:
            *out = WasmRefExtern;
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidType, 0);
    }
    return wbin_ok(bytes + 1);
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
    if (*bytes != 0x60) return wbin_err(WasmDecodeErrInvalidType, 0);
    WasmDecodeResult input_result = wbin_decode_result_type(&bytes[1], &out->input_type);
    if (!wbin_is_ok(input_result)) return input_result;
    return wbin_decode_result_type(input_result.value.next_data, &out->output_type);
}

WasmDecodeResult wbin_decode_types(void *data, WasmModule *wmod) {
    u_int32_t len = 0;
    data = wbin_decode_leb128(data, &len);

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

WasmDecodeResult wbin_decode_limits(void *data, WasmLimits *limits) {
    u_int8_t *bytes = data;
    switch (bytes[0]) {
        case 0x00:
            limits->bounded = false;
            break;
        case 0x01:
            limits->bounded = true;
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidLimit, 0);
    }
    data = wbin_decode_leb128(bytes + 1, &limits->min);
    if (limits->bounded) {
        data = wbin_decode_leb128(data, &limits->max);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_table(void *data, WasmTable *table) {
    WasmDecodeResult ref_result = wbin_decode_reftype(data, &table->reftype);
    if (!wbin_is_ok(ref_result)) return ref_result;
    data = ref_result.value.next_data;
    return wbin_decode_limits(data, &table->limits);
}

WasmDecodeResult wbin_decode_tables(void *data, WasmModule *wmod) {
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmTable decoded_table = { 0 };
        WasmDecodeResult result = wbin_decode_table(data, &decoded_table);
        if (!wbin_is_ok(result)) return result;
        wmod_push_back_table(wmod, &decoded_table);
        data = result.value.next_data;
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_mem(void* data, WasmMemType *mem) {
    return wbin_decode_limits(data, &mem->limits);
}

WasmDecodeResult wbin_decode_mems(void* data, WasmModule *wmod) {
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmMemType mem = { 0 };
        WasmDecodeResult result = wbin_decode_mem(data, &mem);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_mem(wmod, &mem);
        len--;
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_name(void* data,  WasmName *name) {
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);
    name->len = len;
    name->bytes = data;
    return wbin_ok(data + len);
}

WasmDecodeResult wbin_decode_global_mutability(void *data, WasmGlobalMutability *mut) {
    u_int8_t *bytes = data;

    switch (bytes[0]) {
        case 0x00:
            *mut = WasmGlobalConst;
            break;
        case 0x01:
            *mut = WasmGlobalVar;
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidGlobalMutability, 0);
    }

    return wbin_ok(bytes + 1);
}

WasmDecodeResult wbin_decode_global(void *data, WasmGlobalType *global) {
    WasmDecodeResult val_result = wbin_decode_val_type(data, &global->valtype);
    if (!wbin_is_ok(val_result)) return val_result;
    data = val_result.value.next_data;
    return wbin_decode_global_mutability(data, &global->mut);
}

WasmDecodeResult wbin_decode_import_desc(void *data, WasmImportDesc *desc) {
    u_int8_t *bytes = data;

    switch (bytes[0]) {
        case 0x00:
            desc->kind = WasmImportFunc;
            return wbin_ok(wbin_decode_leb128(bytes + 1, &desc->value.func));
        case 0x01:
            desc->kind = WasmImportTable;
            return wbin_decode_table(bytes + 1, &desc->value.table);
        case 0x02:
            desc->kind = WasmImportMem;
            return wbin_decode_mem(bytes + 1, &desc->value.mem);
        case 0x03:
            desc->kind = WasmImportGlobal;
            return wbin_decode_global(bytes + 1, &desc->value.global);
        default:
            return wbin_err(WasmDecodeErrInvalidImport, 0);
    }
}

WasmDecodeResult wbin_decode_import(void *data, WasmImport *import) {
    WasmDecodeResult modname_result = wbin_decode_name(data, &import->module_name);
    if (!wbin_is_ok(modname_result)) return modname_result;
    data = modname_result.value.next_data;
    WasmDecodeResult name_result = wbin_decode_name(data, &import->item_name);
    if (!wbin_is_ok(name_result)) return name_result;
    data = name_result.value.next_data;
    return wbin_decode_import_desc(data, &import->desc);
}

WasmDecodeResult wbin_decode_imports(void *data, WasmModule *wmod) {
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmImport import;
        wmod_import_init(&import);
        WasmDecodeResult result = wbin_decode_import(data, &import);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_import(wmod, &import);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_export_desc(void *data, WasmExportDesc *desc) {
    u_int8_t *bytes = data;

    switch (bytes[0]) {
        case 0x00:
            desc->kind = WasmExportFunc;
            return wbin_ok(wbin_decode_leb128(bytes + 1, &desc->value.func));
        case 0x01:
            desc->kind = WasmExportTable;
            return wbin_ok(wbin_decode_leb128(bytes + 1, &desc->value.table));
        case 0x02:
            desc->kind = WasmExportMem;
            return wbin_ok(wbin_decode_leb128(bytes + 1, &desc->value.mem));
        case 0x03:
            desc->kind = WasmExportGlobal;
            return wbin_ok(wbin_decode_leb128(bytes + 1, &desc->value.global));
        default:
            return wbin_err(WasmDecodeErrInvalidExport, 0);
    }
}

WasmDecodeResult wbin_decode_export(void *data, WasmExport *exp) {
    WasmDecodeResult name_result = wbin_decode_name(data, &exp->name);
    if (!wbin_is_ok(name_result)) return name_result;
    data = name_result.value.next_data;
    return wbin_decode_export_desc(data, &exp->desc);
}

WasmDecodeResult wbin_decode_exports(void *data, WasmModule *wmod) {
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmExport exp;
        wmod_export_init(&exp);
        WasmDecodeResult result = wbin_decode_export(data, &exp);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_export(wmod, &exp);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_start(void *data, WasmModule *wmod) {
    wmod->start.present = true;
    data = wbin_decode_leb128(data, &wmod->start.func_idx);
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_section(WasmSectionId id, void *section, WasmModule *wmod) {
    switch (id) {
        case SectionIdType:
            return wbin_decode_types(section, wmod);
        case SectionIdFunction:
            return wbin_decode_funcs(section, wmod);
        case SectionIdTable:
            return wbin_decode_tables(section, wmod);
        case SectionIdMemory:
            return wbin_decode_mems(section, wmod);
        case SectionIdImport:
            return wbin_decode_imports(section, wmod);
        case SectionIdExport:
            return wbin_decode_exports(section, wmod);
        case SectionIdStart:
            return wbin_decode_start(section, wmod);
        case SectionIdCustom:
        case SectionIdGlobal:
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
        case WasmDecodeErrInvalidImport:
            return "invalid import";
        case WasmDecodeErrInvalidGlobalMutability:
            return "invalid global mutability";
        case WasmDecodeErrInvalidExport:
            return "invalid export";
        default:
            return "unknown error code";
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
