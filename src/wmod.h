
#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include "vec.h"

typedef struct {
    u_int32_t min;
    bool bounded;
    u_int32_t max;
} WasmLimits;

typedef enum {
    WasmNumI32,
    WasmNumI64,
    WasmNumF32,
    WasmNumF64
} WasmNumType;

typedef enum {
    WasmRefFunc,
    WasmRefExtern
} WasmRefType;

typedef enum {
    WasmVecV128
} WasmVecType;

typedef enum {
    WasmValueTypeNum,
    WasmValueTypeVec,
    WasmValueTypeRef
} WasmValueTypeKind;

typedef struct {
    WasmValueTypeKind kind;
    union {
        WasmNumType num;
        WasmVecType vec;
        WasmRefType ref;
    } value;
} WasmValueType;

typedef VEC(WasmValueType) WasmResultType;

typedef struct {
    WasmResultType input_type;
    WasmResultType output_type;
} WasmFuncType;

typedef VEC(WasmFuncType) WasmTypes;
typedef u_int32_t wasm_type_idx_t;

typedef enum {
    WasmInstructionNop
} WasmOpcode;

typedef struct {
    WasmOpcode opcode;
} WasmInstruction;

typedef struct {
    wasm_type_idx_t type_idx;
    VEC(WasmValueType) locals;
    VEC(WasmInstruction) body;
} WasmFunc;

typedef VEC(WasmFunc) WasmFuncs;
typedef u_int32_t wasm_func_idx_t;

typedef struct {
    WasmLimits limits;
    WasmRefType reftype;
} WasmTable;

typedef VEC(WasmTable) WasmTables;
typedef u_int32_t wasm_table_idx_t;

typedef struct {
    WasmLimits limits;
} WasmMemType;

typedef VEC(WasmMemType) WasmMems;
typedef u_int32_t wasm_mem_idx_t;

typedef struct {
    size_t len;
    u_int8_t *bytes;
} WasmName;

typedef enum {
    WasmGlobalVar,
    WasmGlobalConst
} WasmGlobalMutability;

typedef struct {
    WasmGlobalMutability mut;
    WasmValueType valtype;
} WasmGlobalType;

typedef enum {
    WasmImportFunc,
    WasmImportTable,
    WasmImportMem,
    WasmImportGlobal
} WasmImportKind;

typedef struct {
    WasmImportKind kind;
    union {
        wasm_type_idx_t func;
        WasmTable table;
        WasmMemType mem;
        WasmGlobalType global;
    } value;
} WasmImportDesc;

typedef struct {
    WasmName module_name;
    WasmName item_name;
    WasmImportDesc desc;
} WasmImport;

typedef VEC(WasmImport) WasmImports;

typedef Vec WasmGlobals;
typedef u_int32_t wasm_global_idx_t;

typedef Vec WasmElems;
typedef Vec WasmDatas;
typedef Vec WasmStart;

typedef enum {
    WasmExportFunc,
    WasmExportTable,
    WasmExportMem,
    WasmExportGlobal
} WasmExportKind;

typedef struct {
    WasmExportKind kind;
    union {
        wasm_func_idx_t func;
        wasm_table_idx_t table;
        wasm_mem_idx_t mem;
        wasm_global_idx_t global;
    } value;
} WasmExportDesc;

typedef struct {
    WasmName name;
    WasmExportDesc desc;
} WasmExport;

typedef VEC(WasmExport) WasmExports;

typedef Vec WasmCustoms;

typedef struct {
    u_int32_t version;
} WasmMeta;

typedef struct {
    WasmTypes types;
    WasmFuncs funcs;
    WasmTables tables;
    WasmMems mems;
    WasmGlobals globals;
    WasmElems elems;
    WasmDatas datas;
    WasmStart start;
    WasmImports imports;
    WasmExports exports;
    WasmCustoms customs;
    WasmMeta meta;
} WasmModule;

void wmod_init(WasmModule *wmod);
void wmod_dump(WasmModule *wmod);

void wmod_name_init(WasmName *name);
void wmod_func_type_init(WasmFuncType *type);
void wmod_func_init(WasmFunc *func);
void wmod_import_init(WasmImport *import);
void wmod_export_init(WasmExport *exp);

size_t wmod_result_type_push_back(WasmResultType *type, WasmValueType *valtype);

wasm_type_idx_t wmod_push_back_type(WasmModule *wmod, WasmFuncType *type);
wasm_func_idx_t wmod_push_back_func(WasmModule *wmod, WasmFunc *func);
wasm_table_idx_t wmod_push_back_table(WasmModule *wmod, WasmTable *table);
wasm_mem_idx_t wmod_push_back_mem(WasmModule *wmod, WasmMemType *mem);
void wmod_push_back_import(WasmModule *wmod, WasmImport *import);
void wmod_push_back_export(WasmModule *wmod, WasmExport *exp);
