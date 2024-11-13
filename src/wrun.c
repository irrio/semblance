
#include "wrun.h"

const u_int32_t WMEM_PAGE_SIZE = 65536;

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

wasm_table_addr_t wrun_store_alloc_table(WasmStore *store, WasmTable *table, WasmRefValue initval) {
    WasmTableInst tinst;
    tinst.tabletype = *table;
    vec_init_with_size(&tinst.elems, sizeof(WasmRefValue), table->limits.min, &initval);
    return vec_push_back(&store->tables, sizeof(WasmTableInst), &tinst) + 1;
}

wasm_mem_addr_t wrun_store_alloc_mem(WasmStore *store, WasmMemType *mem) {
    WasmMemInst minst;
    minst.memtype = *mem;
    vec_init_with_zeros(&minst.data, 1, WMEM_PAGE_SIZE * mem->limits.min);
    return vec_push_back(&store->mems, sizeof(WasmMemInst), &minst) + 1;
}

wasm_global_addr_t wrun_store_alloc_global(WasmStore *store, WasmGlobalType *globaltype, WasmValue val) {
    WasmGlobalInst ginst;
    ginst.globaltype = *globaltype;
    ginst.val = val;
    return vec_push_back(&store->globals, sizeof(WasmGlobalInst), &ginst) + 1;
}

wasm_elem_addr_t wrun_store_alloc_elem(WasmStore *store, WasmElem *elem, VEC(WasmRefValue) *references) {
    WasmElemInst einst;
    vec_init(&einst.elem);
    vec_clone(references, &einst.elem, sizeof(WasmRefValue));
    einst.reftype = elem->reftype;
    return vec_push_back(&store->elems, sizeof(WasmElemInst), &einst) + 1;
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
        wasm_table_addr_t tableaddr = wrun_store_alloc_table(store, table, 0);
        vec_push_back(&tableaddrs, sizeof(wasm_table_addr_t), &tableaddr);
    }

    VEC(wasm_table_addr_t) memaddrs;
    vec_init(&memaddrs);
    for (size_t i = 0; i < wmod->mems.len; i++) {
        WasmMemType *mem = wmod->mems.ptr + (i * sizeof(WasmMemType));
        wasm_mem_addr_t memaddr = wrun_store_alloc_mem(store, mem);
        vec_push_back(&memaddrs, sizeof(wasm_mem_addr_t), &memaddr);
    }

    VEC(wasm_global_addr_t) globaladdrs;
    vec_init(&globaladdrs);
    for (size_t i = 0; i < wmod->globals.len; i++) {
        WasmGlobal *global = wmod->globals.ptr + (i * sizeof(WasmGlobal));
        WasmValue initval;
        wrun_value_default(global->globaltype.valtype, &initval);
        wasm_global_addr_t globaladdr = wrun_store_alloc_global(store, &global->globaltype, initval);
        vec_push_back(&globaladdrs, sizeof(wasm_global_addr_t), &globaladdr);
    }

    VEC(wasm_elem_addr_t) elemaddrs;
    vec_init(&elemaddrs);
    for (size_t i = 0; i < wmod->elems.len; i++) {
        WasmElem *elem = wmod->elems.ptr + (i * sizeof(WasmElem));
        Vec references;
        vec_init(&references);
        wasm_elem_addr_t elemaddr = wrun_store_alloc_elem(store, elem, &references);
        vec_push_back(&elemaddrs, sizeof(wasm_elem_addr_t), &elemaddr);
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
