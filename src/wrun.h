
#pragma once

#include <stdint.h>
#include "vec.h"
#include "wmod.h"

typedef u_int32_t wasm_addr_t;

typedef wasm_addr_t wasm_func_addr_t;
typedef wasm_addr_t wasm_table_addr_t;
typedef wasm_addr_t wasm_mem_addr_t;
typedef wasm_addr_t wasm_global_addr_t;
typedef wasm_addr_t wasm_elem_addr_t;
typedef wasm_addr_t wasm_data_addr_t;
typedef wasm_addr_t wasm_extern_addr_t;

typedef union {
    int32_t i32;
    int64_t i64;
    float f32;
    double f64;
} WasmNumValue;

typedef u_int8_t WasmVecValue [8];

typedef wasm_addr_t WasmRefValue;

typedef union {
    WasmNumValue num;
    WasmVecValue vec;
    WasmRefValue ref;
} WasmValue;

void wrun_value_default(WasmValueType valtype, WasmValue *value);

typedef enum {
    Ok,
    Trap
} WasmResultKind;

typedef struct {
    WasmResultKind kind;
    VEC(WasmValue) values;
} WasmResult;

void wrun_result_init(WasmResult *result);

typedef enum {
    WasmExternValFunc,
    WasmExternValTable,
    WasmExternValMem,
    WasmExternValGlobal
} WasmExternValKind;

typedef struct {
    WasmExternValKind kind;
    union {
        wasm_func_addr_t func;
        wasm_table_addr_t table;
        wasm_mem_addr_t mem;
        wasm_global_addr_t global;
    } val;
} WasmExternVal;

typedef struct {
    WasmName name;
    WasmExternVal val;
} WasmExportInst;

typedef struct {
    WasmFuncType *types;
    wasm_func_addr_t *funcaddrs;
    wasm_table_addr_t *tableaddrs;
    wasm_mem_addr_t *memaddrs;
    wasm_global_addr_t *globaladdrs;
    wasm_elem_addr_t *elemaddrs;
    wasm_data_addr_t *dataaddrs;
    WasmExportInst *exports;
} WasmModuleInst;

typedef enum {
    WasmFuncInstWasm,
    WasmFuncInstHost,
} WasmFuncInstKind;

typedef struct {
    WasmFuncType functype;
    WasmFuncInstKind kind;
    union {
        struct {
            WasmModuleInst *module;
            WasmFunc *func;
        } wasmfunc;
        void *hostfunc;
    } val;
} WasmFuncInst;

typedef struct {
    WasmTable tabletype;
    VEC(WasmRefValue) elems;
} WasmTableInst;

typedef struct {
    WasmMemType memtype;
    VEC(u_int8_t) data;
} WasmMemInst;

typedef struct {
    WasmGlobalType globaltype;
    WasmValue val;
} WasmGlobalInst;

typedef struct {
    WasmRefType reftype;
    VEC(WasmRefType) elem;
} WasmElemInst;

typedef struct {
    VEC(u_int8_t) data;
} WasmDataInst;

typedef struct {
    VEC(WasmFuncInst) funcs;
    VEC(WasmTableInst) tables;
    VEC(WasmMemInst) mems;
    VEC(WasmGlobalInst) globals;
    VEC(WasmElemInst) elems;
    VEC(WasmDataInst) datas;
} WasmStore;

void wrun_store_init(WasmStore *store);
wasm_func_addr_t wrun_store_alloc_func(WasmStore *store, WasmModuleInst *winst, WasmFunc *func);
wasm_table_addr_t wrun_store_alloc_table(WasmStore *store, WasmTable *table, WasmRefValue initval);
wasm_mem_addr_t wrun_store_alloc_mem(WasmStore *store, WasmMemType *mem);
void wrun_instantiate_module(WasmModule *wmod, WasmStore *store, WasmModuleInst *winst);

typedef struct {
    u_int32_t argument_arity;
    WasmInstruction *instr;
} WasmLabel;

typedef struct {
    u_int32_t return_arity;
    VEC(WasmValue) locals;
    WasmModuleInst *inst;
} WasmActivation;

typedef enum {
    WasmStackEntryValue,
    WasmStackEntryLabel,
    WasmStackEntryActivation,
} WasmStackEntryKind;

typedef struct {
    WasmStackEntryKind kind;
    union {
        WasmValue val;
        WasmLabel label;
        WasmActivation activation;
    } entry;
} WasmStackEntry;

typedef struct {
    VEC(WasmStackEntry) entries;
} WasmStack;

void wrun_stack_init(WasmStack *stack);
size_t wrun_stack_push(WasmStack *stack, WasmStackEntry *entry);
size_t wrun_stack_push_auxiliary_frame(WasmStack *stack, WasmModuleInst *winst);
bool wrun_stack_pop(WasmStack *stack, WasmStackEntry *out);
