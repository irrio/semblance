
#pragma once

#include <stdint.h>
#include "vec.h"
#include "wmod.h"

typedef uint32_t wasm_addr_t;

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

typedef uint8_t WasmVecValue [8];

typedef wasm_addr_t WasmRefValue;

typedef union {
    WasmNumValue num;
    WasmVecValue vec;
    WasmRefValue ref;
} WasmValue;

void wrun_value_default(WasmValueType valtype, WasmValue *value);
void wrun_value_dump(WasmValueType valtype, WasmValue *value);

typedef enum {
    Ok,
    Trap
} WasmResultKind;

typedef struct {
    WasmResultKind kind;
    VEC(WasmValue) values;
} WasmResult;

typedef struct {
    WasmResultType result_type;
    WasmResult result;
} DynamicWasmResult;

void wrun_result_init(WasmResult *result);
void wrun_result_dump(WasmResult *result, WasmResultType *type);
void wrun_result_dump_dynamic(DynamicWasmResult *result);

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
    VEC(wasm_func_addr_t) funcaddrs;
    VEC(wasm_table_addr_t) tableaddrs;
    VEC(wasm_mem_addr_t) memaddrs;
    VEC(wasm_global_addr_t) globaladdrs;
    VEC(wasm_elem_addr_t) elemaddrs;
    VEC(wasm_data_addr_t) dataaddrs;
    VEC(WasmExportInst) exports;
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
    VEC(WasmRefValue) elem;
} WasmElemInst;

typedef struct {
    uint8_t *bytes;
    size_t len;
} WasmDataInst;

typedef struct {
    VEC(WasmFuncInst) funcs;
    VEC(WasmTableInst) tables;
    VEC(WasmMemInst) mems;
    VEC(WasmGlobalInst) globals;
    VEC(WasmElemInst) elems;
    VEC(WasmDataInst) datas;
} WasmStore;

typedef VEC(WasmValue) (*WasmHostFunc) (WasmStore*, VEC(WasmValue)*);

void wrun_store_init(WasmStore *store);
wasm_func_addr_t wrun_store_alloc_hostfunc(WasmStore *store, WasmFuncType functype, WasmHostFunc fptr);
wasm_func_addr_t wrun_store_alloc_func(WasmStore *store, WasmModuleInst *winst, WasmFunc *func);
wasm_table_addr_t wrun_store_alloc_table(WasmStore *store, WasmTable *table, WasmRefValue initval);
wasm_mem_addr_t wrun_store_alloc_mem(WasmStore *store, WasmMemType *mem);
wasm_global_addr_t wrun_store_alloc_global(WasmStore *store, WasmGlobalType *globaltype, WasmValue val);
wasm_elem_addr_t wrun_store_alloc_elem(WasmStore *store, WasmElem *elem, VEC(WasmRefValue) *references);
wasm_data_addr_t wrun_store_alloc_data(WasmStore *store, WasmData *wdata);

typedef struct {
    VEC(WasmExternVal) imports;
    VEC(WasmValue) globalinit;
    VEC(VEC(WasmRefValue)) references;
} WasmInitParams;

void wrun_init_params_init(WasmInitParams *params, VEC(WasmExternVal) *imports);
void wrun_apply_imports(VEC(WasmExternVal) *imports, WasmModuleInst *winst);
WasmModuleInst *wrun_store_alloc_module(WasmStore *store, WasmModule *wmod, WasmInitParams *params);
WasmModuleInst *wrun_instantiate_module(WasmModule *wmod, WasmStore *store, VEC(WasmExternVal) *imports);

typedef struct {
    uint32_t argument_arity;
    WasmInstruction *instr;
} WasmLabel;

typedef struct {
    uint32_t return_arity;
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
bool wrun_stack_pop_and_drop(WasmStack *stack);

WasmResultKind wrun_eval_expr(WasmStore *store, WasmStack *stack, WasmInstruction *expr, WasmValue *wval);
WasmResultKind wrun_exec_expr(WasmStore *store, WasmStack *stack, WasmInstruction *expr);

WasmExternVal wrun_resolve_export(WasmModuleInst *winst, char *name);
DynamicWasmResult wrun_invoke_func(wasm_func_addr_t funcaddr, VEC(WasmValue) *args, WasmStore *store);
