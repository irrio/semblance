
#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "leb128.h"
#include "wmod.h"

typedef enum __attribute__((packed)) {
    SectionIdCustom = 0,
    SectionIdType = 1,
    SectionIdImport = 2,
    SectionIdFunction = 3,
    SectionIdTable = 4,
    SectionIdMemory = 5,
    SectionIdGlobal = 6,
    SectionIdExport = 7,
    SectionIdStart = 8,
    SectionIdElement = 9,
    SectionIdCode = 10,
    SectionIdData = 11,
    SectionIdDataCount = 12
} WasmSectionId;

typedef u_leb128_prefixed WasmSectionData;

typedef struct __attribute__((packed)) {
    WasmSectionId section_id;
    WasmSectionData data;
} WasmSectionHeader;

typedef struct __attribute__((packed)) {
    u_int8_t magic_bytes[4];
    u_int32_t version;
    WasmSectionHeader sections[];
} WasmHeader;

typedef enum {
    WasmDecodeOk= 0,
    WasmDecodeErrIo,
    WasmDecodeErrMagicBytes,
    WasmDecodeErrUnsupportedVersion,
    WasmDecodeErrOom,
    WasmDecodeErrLeb128,
    WasmDecodeErrUnknownSectionId,
} WasmDecodeState;

typedef struct {
    WasmDecodeState state;
    int cause;
} WasmDecodeResult;

WasmDecodeResult wbin_read_module(char *path, WasmModule *wmod);
WasmDecodeResult wbin_decode_module(size_t size, WasmHeader *header, WasmModule *wmod);

char *wbin_explain_error_code(WasmDecodeResult result);
char *wbin_explain_error_cause(WasmDecodeResult result);
bool wbin_is_ok(WasmDecodeResult result);
bool wbin_error_has_cause(WasmDecodeResult result);
