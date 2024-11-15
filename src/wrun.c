
#include "wrun.h"
#include <stdlib.h>
#include <assert.h>

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

wasm_data_addr_t wrun_store_alloc_data(WasmStore *store, WasmData *wdata) {
    WasmDataInst dinst;
    dinst.bytes= wdata->bytes;
    return vec_push_back(&store->datas, sizeof(WasmDataInst), &dinst) + 1;
}

void wrun_init_params_init(WasmInitParams *params) {
    vec_init(&params->globalinit);
    vec_init(&params->imports);
    vec_init(&params->references);
}

void wrun_decompose_imports(VEC(WasmExternVal) *imports, WasmDecomposedImports *out) {
    vec_init(&out->funcs);
    vec_init(&out->globals);
    vec_init(&out->mems);
    vec_init(&out->tables);

    for (size_t i = 0; i < imports->len; i++) {
        WasmExternVal *import = vec_at(imports, sizeof(WasmExternVal), i);
        switch (import->kind) {
            case WasmExternValFunc:
                vec_push_back(&out->funcs, sizeof(wasm_func_addr_t), &import->val.func);
                break;
            case WasmExternValMem:
                vec_push_back(&out->mems, sizeof(wasm_mem_addr_t), &import->val.mem);
                break;
            case WasmExternValGlobal:
                vec_push_back(&out->globals, sizeof(wasm_global_addr_t), &import->val.global);
                break;
            case WasmExternValTable:
                vec_push_back(&out->tables, sizeof(wasm_table_addr_t), &import->val.table);
                break;
        }
    }
}

void wrun_free_decomposed_imports(WasmDecomposedImports *decomposed) {
    vec_free(&decomposed->funcs);
    vec_free(&decomposed->mems);
    vec_free(&decomposed->tables);
    vec_free(&decomposed->globals);
}

WasmModuleInst *wrun_store_alloc_module(WasmStore *store, WasmModule *wmod, WasmInitParams *params) {
    assert(params->globalinit.len == wmod->globals.len);
    assert(params->imports.len == wmod->imports.len);
    assert(params->references.len == wmod->elems.len);

    WasmModuleInst *winst = malloc(sizeof(WasmModuleInst));
    winst->types = wmod->types.ptr;

    VEC(wasm_func_addr_t) funcaddrs;
    vec_init(&funcaddrs);
    for (size_t i = 0; i < wmod->funcs.len; i++) {
        WasmFunc *func = wmod->funcs.ptr + (i * sizeof(WasmFunc));
        wasm_func_addr_t funcaddr = wrun_store_alloc_func(store, winst, func);
        vec_push_back(&funcaddrs, sizeof(wasm_func_addr_t), &funcaddr);
    }
    winst->funcaddrs = funcaddrs.ptr;

    VEC(wasm_table_addr_t) tableaddrs;
    vec_init(&tableaddrs);
    for (size_t i = 0; i < wmod->tables.len; i++) {
        WasmTable *table = wmod->tables.ptr + (i * sizeof(WasmTable));
        wasm_table_addr_t tableaddr = wrun_store_alloc_table(store, table, 0);
        vec_push_back(&tableaddrs, sizeof(wasm_table_addr_t), &tableaddr);
    }
    winst->tableaddrs = tableaddrs.ptr;

    VEC(wasm_table_addr_t) memaddrs;
    vec_init(&memaddrs);
    for (size_t i = 0; i < wmod->mems.len; i++) {
        WasmMemType *mem = wmod->mems.ptr + (i * sizeof(WasmMemType));
        wasm_mem_addr_t memaddr = wrun_store_alloc_mem(store, mem);
        vec_push_back(&memaddrs, sizeof(wasm_mem_addr_t), &memaddr);
    }
    winst->memaddrs = memaddrs.ptr;

    VEC(wasm_global_addr_t) globaladdrs;
    vec_init(&globaladdrs);
    for (size_t i = 0; i < wmod->globals.len; i++) {
        WasmGlobal *global = wmod->globals.ptr + (i * sizeof(WasmGlobal));
        WasmValue initval = *(WasmValue *)vec_at(&params->globalinit, sizeof(WasmValue), i);
        wasm_global_addr_t globaladdr = wrun_store_alloc_global(store, &global->globaltype, initval);
        vec_push_back(&globaladdrs, sizeof(wasm_global_addr_t), &globaladdr);
    }
    winst->globaladdrs = globaladdrs.ptr;

    VEC(wasm_elem_addr_t) elemaddrs;
    vec_init(&elemaddrs);
    for (size_t i = 0; i < wmod->elems.len; i++) {
        WasmElem *elem = wmod->elems.ptr + (i * sizeof(WasmElem));
        Vec *references = vec_at(&params->references, sizeof(Vec), i);
        wasm_elem_addr_t elemaddr = wrun_store_alloc_elem(store, elem, references);
        vec_push_back(&elemaddrs, sizeof(wasm_elem_addr_t), &elemaddr);
    }
    winst->elemaddrs = elemaddrs.ptr;

    VEC(wasm_data_addr_t) dataaddrs;
    vec_init(&dataaddrs);
    for (size_t i = 0; i < wmod->datas.len; i++) {
        WasmData *wdata = wmod->datas.ptr + (i * sizeof(WasmData));
        wasm_data_addr_t dataaddr = wrun_store_alloc_data(store, wdata);
        vec_push_back(&dataaddrs, sizeof(wasm_data_addr_t), &dataaddr);
    }
    winst->dataaddrs = dataaddrs.ptr;

    WasmDecomposedImports decomposed;
    wrun_decompose_imports(&params->imports, &decomposed);
    VEC(WasmExportInst) exports;
    vec_init(&exports);
    for (size_t i = 0; i < wmod->exports.len; i++) {
        WasmExport *wexp = wmod->exports.ptr + (i * sizeof(WasmExport));
        WasmExportInst inst;
        inst.name = wexp->name;
        switch (wexp->desc.kind) {
            case WasmExportMem: {
                inst.val.kind = WasmExternValMem;
                wasm_mem_idx_t idx = wexp->desc.value.mem;
                if (idx < decomposed.mems.len) {
                    inst.val.val.mem = *(wasm_mem_addr_t*)vec_at(&decomposed.mems, sizeof(wasm_mem_addr_t), idx);
                } else {
                    inst.val.val.mem = *(wasm_mem_addr_t*)vec_at(&memaddrs, sizeof(wasm_mem_addr_t), idx - decomposed.mems.len);
                }
                break;
            }
            case WasmExportFunc: {
                inst.val.kind = WasmExternValFunc;
                wasm_func_idx_t idx = wexp->desc.value.func;
                if (idx < decomposed.funcs.len) {
                    inst.val.val.func = *(wasm_func_addr_t*)vec_at(&decomposed.funcs, sizeof(wasm_func_addr_t), idx);
                } else {
                    inst.val.val.func = *(wasm_func_addr_t*)vec_at(&funcaddrs, sizeof(wasm_func_addr_t), idx - decomposed.funcs.len);
                }
                break;
            }
            case WasmExportTable: {
                inst.val.kind = WasmExternValTable;
                wasm_table_idx_t idx = wexp->desc.value.table;
                if (idx < decomposed.tables.len) {
                    inst.val.val.table = *(wasm_table_addr_t*)vec_at(&decomposed.tables, sizeof(wasm_table_addr_t), idx);
                } else {
                    inst.val.val.table = *(wasm_table_addr_t*)vec_at(&tableaddrs, sizeof(wasm_table_addr_t), idx - decomposed.tables.len);
                }
                break;
            }
            case WasmExportGlobal: {
                inst.val.kind = WasmExternValGlobal;
                wasm_global_idx_t idx = wexp->desc.value.global;
                if (idx < decomposed.globals.len) {
                    inst.val.val.global = *(wasm_global_addr_t*)vec_at(&decomposed.globals, sizeof(wasm_global_addr_t), idx);
                } else {
                    inst.val.val.global = *(wasm_global_addr_t*)vec_at(&globaladdrs, sizeof(wasm_global_addr_t), idx - decomposed.globals.len);
                }
                break;
            }
        }
        vec_push_back(&exports, sizeof(WasmExportInst), &inst);
    }
    wrun_free_decomposed_imports(&decomposed);
    winst->exports = exports.ptr;
    return winst;
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
