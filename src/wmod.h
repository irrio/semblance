
#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include "vec.h"

typedef u_int32_t wasm_func_idx_t;
typedef u_int32_t wasm_type_idx_t;
typedef u_int32_t wasm_label_idx_t;
typedef u_int32_t wasm_global_idx_t;
typedef u_int32_t wasm_local_idx_t;
typedef u_int32_t wasm_table_idx_t;
typedef u_int32_t wasm_elem_idx_t;
typedef u_int32_t wasm_data_idx_t;
typedef u_int32_t wasm_mem_idx_t;

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

typedef VEC(WasmInstruction) WasmExpr;

typedef enum {
    WasmBlockTypeEmpty,
    WasmBlockTypeIdx,
    WasmBlockTypeVal,
} WasmBlockTypeKind;

typedef struct {
    WasmBlockTypeKind kind;
    union {
        wasm_type_idx_t typeidx;
        WasmValueType valtype;
    } value;
} WasmBlockType;

typedef struct {
    WasmBlockType blocktype;
    WasmExpr expr;
} WasmBlockParams;

typedef struct {
    WasmBlockType blocktype;
    WasmExpr then_body;
    WasmExpr else_body;
} WasmIfParams;


typedef struct {
    wasm_label_idx_t label;
} WasmBreakParams;

typedef struct {
    VEC(wasm_label_idx_t) labels;
    wasm_label_idx_t default_label;
} WasmBreakTableParams;

typedef struct {
    wasm_func_idx_t funcidx;
} WasmCallParams;

typedef struct {
    wasm_table_idx_t tableidx;
    wasm_type_idx_t typeidx;
} WasmCallIndirectParams;

typedef struct {
    WasmRefType reftype;
} WasmRefNullParams;

typedef struct {
    wasm_func_idx_t funcidx;
} WasmRefFuncParams;

typedef struct {
    VEC(WasmValueType) valuetypes;
} WasmSelectParams;

typedef struct {
    union {
        wasm_local_idx_t local;
        wasm_global_idx_t global;
    } idx;
} WasmVarParams;

typedef struct {
    wasm_table_idx_t tableidx;
} WasmTableParams;

typedef struct {
    wasm_table_idx_t src;
    wasm_table_idx_t dst;
} WasmTableCopyParams;

typedef struct {
    wasm_table_idx_t tableidx;
    wasm_elem_idx_t elemidx;
} WasmTableInitParams;

typedef struct {
    wasm_elem_idx_t elemidx;
} WasmElemDropParams;

typedef struct {
    u_int32_t align;
    u_int32_t offset;
} WasmMemArg;

typedef struct {
    wasm_data_idx_t dataidx;
} WasmMemoryInitParams;

typedef enum {
    WasmOpUnreachable,
    WasmOpNop,
    WasmOpBlock,
    WasmOpLoop,
    WasmOpIf,
    WasmOpElse,
    WasmOpBreak,
    WasmOpBreakIf,
    WasmOpBreakTable,
    WasmOpReturn,
    WasmOpCall,
    WasmOpCallIndirect,
    WasmOpExprEnd,
    WasmOpRefNull,
    WasmOpRefIsNull,
    WasmOpRefFunc,
    WasmOpDrop,
    WasmOpSelect,
    WasmOpLocalGet,
    WasmOpLocalSet,
    WasmOpLocalTee,
    WasmOpGlobalGet,
    WasmOpGlobalSet,
    WasmOpTableGet,
    WasmOpTableSet,
    WasmOpTableSize,
    WasmOpTableGrow,
    WasmOpTableFill,
    WasmOpTableCopy,
    WasmOpTableInit,
    WasmOpElemDrop,
    WasmOpI32Load,
    WasmOpI64Load,
    WasmOpF32Load,
    WasmOpF64Load,
    WasmOpI32Load8_s,
    WasmOpI32Load8_u,
    WasmOpI32Load16_s,
    WasmOpI32Load16_u,
    WasmOpI64Load8_s,
    WasmOpI64Load8_u,
    WasmOpI64Load16_s,
    WasmOpI64Load16_u,
    WasmOpI64Load32_s,
    WasmOpI64Load32_u,
    WasmOpI32Store,
    WasmOpI64Store,
    WasmOpF32Store,
    WasmOpF64Store,
    WasmOpI32Store8,
    WasmOpI32Store16,
    WasmOpI64Store8,
    WasmOpI64Store16,
    WasmOpI64Store32,
    WasmOpMemorySize,
    WasmOpMemoryGrow,
    WasmOpMemoryInit,
    WasmOpDataDrop,
    WasmOpMemoryCopy,
    WasmOpMemoryFill,
} WasmOpcode;

typedef struct {
    WasmOpcode opcode;
    union {
        WasmBlockParams block;
        WasmIfParams _if;
        WasmBreakParams _break;
        WasmBreakTableParams break_table;
        WasmCallParams call;
        WasmCallIndirectParams call_indirect;
        WasmRefNullParams ref_null;
        WasmRefFuncParams ref_func;
        WasmSelectParams select;
        WasmVarParams var;
        WasmTableParams table;
        WasmTableCopyParams table_copy;
        WasmTableInitParams table_init;
        WasmElemDropParams elem_drop;
        WasmMemArg memarg;
        WasmMemoryInitParams mem_init;
    } params;
} WasmInstruction;

typedef struct {
    wasm_type_idx_t type_idx;
    VEC(WasmValueType) locals;
    WasmExpr body;
} WasmFunc;

typedef VEC(WasmFunc) WasmFuncs;

typedef struct {
    WasmLimits limits;
    WasmRefType reftype;
} WasmTable;

typedef VEC(WasmTable) WasmTables;

typedef struct {
    WasmLimits limits;
} WasmMemType;

typedef VEC(WasmMemType) WasmMems;

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

typedef Vec WasmElems;
typedef Vec WasmDatas;

typedef struct {
    bool present;
    wasm_func_idx_t func_idx;
} WasmStart;

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

void wmod_func_push_back_locals(WasmFunc *func, u_int32_t n, WasmValueType *val);
void wmod_expr_push_back_instruction(WasmExpr *expr, WasmInstruction *ins);
void wmod_instr_init(WasmInstruction *instr, WasmOpcode opcode);
