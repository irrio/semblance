
#pragma once

#include <sys/_types/_u_int32_t.h>
#include <sys/types.h>

extern const u_int8_t WMOD_SECTION_ID_CUSTOM;
extern const u_int8_t WMOD_SECTION_ID_TYPE;
extern const u_int8_t WMOD_SECTION_ID_IMPORT;
extern const u_int8_t WMOD_SECTION_ID_FUNCTION;
extern const u_int8_t WMOD_SECTION_ID_TABLE;
extern const u_int8_t WMOD_SECTION_ID_MEMORY;
extern const u_int8_t WMOD_SECTION_ID_GLOBAL;
extern const u_int8_t WMOD_SECTION_ID_EXPORT;
extern const u_int8_t WMOD_SECTION_ID_START;
extern const u_int8_t WMOD_SECTION_ID_ELEMENT;
extern const u_int8_t WMOD_SECTION_ID_CODE;
extern const u_int8_t WMOD_SECTION_ID_DATA;
extern const u_int8_t WMOD_SECTION_ID_DATA_COUNT;

typedef struct __attribute__((packed)) {
    u_int8_t section_id;
    u_int32_t size;
    u_int8_t data[];
} WasmSectionHeader;

typedef struct __attribute__((packed)) {
    char magic_bytes[4];
    u_int32_t version;
    WasmSectionHeader sections[];
} WasmHeader;

typedef struct {
    u_int8_t section_id;
    u_int32_t size;
    u_int8_t *data;
} WasmSection;

typedef struct {
    off_t total_size;
    WasmHeader *raw_data;
    size_t num_sections;
    WasmSection sections[];
} WasmModule;

typedef struct {
    int wmod_err;
    int cause;
} WmodErr;

WasmModule *wmod_read(char *path, WmodErr *err);

int wmod_failed(WmodErr);
char *wmod_str_error(WmodErr err);
char *wmod_str_error_cause(WmodErr err);
