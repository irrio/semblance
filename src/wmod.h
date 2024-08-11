
#pragma once

#include <stdint.h>
#include <stddef.h>
#include "vec.h"

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
typedef size_t wasm_type_idx_t;

typedef Vec WasmFuncs;
typedef Vec WasmTables;
typedef Vec WasmMems;
typedef Vec WasmGlobals;
typedef Vec WasmElems;
typedef Vec WasmDatas;
typedef Vec WasmStart;
typedef Vec WasmImports;
typedef Vec WasmExports;
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

void wmod_func_type_init(WasmFuncType *type);

size_t wmod_result_type_push_back(WasmResultType *type, WasmValueType *valtype);

wasm_type_idx_t wmod_push_back_type(WasmModule *wmod, WasmFuncType *type);
