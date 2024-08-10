
#pragma once

#include <stdint.h>

typedef u_int8_t WasmTypes;
typedef u_int8_t WasmFuncs;
typedef u_int8_t WasmTables;
typedef u_int8_t WasmMems;
typedef u_int8_t WasmGlobals;
typedef u_int8_t WasmElems;
typedef u_int8_t WasmDatas;
typedef u_int8_t WasmStart;
typedef u_int8_t WasmImports;
typedef u_int8_t WasmExports;
typedef u_int8_t WasmCustoms;

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
