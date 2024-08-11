
#pragma once

#include <stdint.h>
#include <stddef.h>
#include "vec.h"

typedef enum {
    WasmValueTypeNum,
    WasmValueTypeVec,
    WasmValueTypeRef
} WasmValueTypeKind;

typedef struct {
    WasmValueTypeKind kind;
} WasmValueType;

typedef VEC(WasmValueType) WasmResultType;

typedef struct {
    WasmResultType input_type;
    WasmResultType output_type;
} WasmFuncType;

typedef VEC(WasmFuncType) WasmTypes;
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
} WasmModule;

void wmod_init(WasmModule *wmod);
