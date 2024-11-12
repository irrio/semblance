
#include "wrun.h"

void wrun_num_default(WasmNumType numtype, WasmNumValue *num) {
    switch (numtype) {
        case WasmNumI32:
            num->i32 = 0;
            break;
        case WasmNumI64:
            num->i64 = 0;
            break;
        case WasmNumF32:
            num->f32 = 0;
            break;
        case WasmNumF64:
            num->f64 = 0;
            break;
    }
}

void wrun_ref_default(WasmRefType reftype, WasmRefValue *ref) {
    *ref = 0;
}

void wrun_vec_default(WasmVecType vectype, WasmVecValue *vec) {
    switch (vectype) {
        case WasmVecV128:
            for (size_t i = 0; i < 8; i++) {
                *vec[i] = 0;
            }
            break;
    }
}

void wrun_value_default(WasmValueType valtype, WasmValue *value) {
    switch (valtype.kind) {
        case WasmValueTypeNum:
            return wrun_num_default(valtype.value.num, &value->num);
        case WasmValueTypeRef:
            return wrun_ref_default(valtype.value.ref, &value->ref);
        case WasmValueTypeVec:
            return wrun_vec_default(valtype.value.vec, &value->vec);
    }
}

void wrun_result_init(WasmResult *result) {
    vec_init(&result->values);
}

void wrun_store_init(WasmStore *store) {
    vec_init(&store->funcs);
    vec_init(&store->tables);
    vec_init(&store->mems);
    vec_init(&store->globals);
    vec_init(&store->elems);
    vec_init(&store->datas);
}

wasm_func_addr_t wrun_store_alloc_func(WasmStore *store, WasmModuleInst *winst, WasmFunc *func) {
    WasmFuncInst finst;
    finst.functype = winst->types[func->type_idx];
    finst.kind = WasmFuncInstWasm;
    finst.val.wasmfunc.module = winst;
    finst.val.wasmfunc.func = func;
    return vec_push_back(&store->funcs, sizeof(WasmFuncInst), &finst) + 1;
}

wasm_table_addr_t wrun_store_alloc_table(WasmStore *store, WasmModuleInst *winst, WasmTable *table, WasmRefValue initval) {
    WasmTableInst tinst;
    tinst.tabletype = *table;
    vec_init_with_size(&tinst.elems, sizeof(WasmRefValue), table->limits.min, &initval);
    return vec_push_back(&store->tables, sizeof(WasmTableInst), &tinst) + 1;
}

void wrun_instantiate_module(WasmModule *wmod, WasmStore *store, WasmModuleInst *winst) {
    winst->types = wmod->types.ptr;

    VEC(wasm_func_addr_t) funcaddrs;
    vec_init(&funcaddrs);
    for (size_t i = 0; i < wmod->funcs.len; i++) {
        WasmFunc *func = wmod->funcs.ptr + (i * sizeof(WasmFunc));
        wasm_func_addr_t funcaddr = wrun_store_alloc_func(store, winst, func);
        vec_push_back(&funcaddrs, sizeof(wasm_func_addr_t), &funcaddr);
    }

    VEC(wasm_table_addr_t) tableaddrs;
    vec_init(&tableaddrs);
    for (size_t i = 0; i < wmod->tables.len; i++) {
        WasmTable *table = wmod->tables.ptr + (i * sizeof(WasmTable));
        wasm_table_addr_t tableaddr = wrun_store_alloc_table(store, winst, table, 0);
        vec_push_back(&tableaddrs, sizeof(wasm_table_addr_t), &tableaddr);
    }
}

void wrun_stack_init(WasmStack *stack) {
    vec_init(&stack->entries);
}

size_t wrun_stack_push(WasmStack *stack, WasmStackEntry *entry) {
    return vec_push_back(&stack->entries, sizeof(WasmStackEntry), entry);
}

size_t wrun_stack_push_auxiliary_frame(WasmStack *stack, WasmModuleInst *winst) {
    WasmStackEntry frame;
    frame.kind = WasmStackEntryActivation;
    frame.entry.activation.return_arity = 0;
    frame.entry.activation.inst = winst;
    vec_init(&frame.entry.activation.locals);
    return wrun_stack_push(stack, &frame);
}

bool wrun_stack_pop(WasmStack *stack, WasmStackEntry *out) {
    return vec_pop_back(&stack->entries, sizeof(WasmStackEntry), out);
}
