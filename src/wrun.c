
#include "wrun.h"
#include "wmod.h"
#include <stdio.h>
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

void wrun_store_alloc_funcs(WasmStore *store, WasmModuleInst *winst, VEC(WasmFunc) *funcs) {
    for (size_t i = 0; i < funcs->len; i++) {
        WasmFunc *func = funcs->ptr + (i * sizeof(WasmFunc));
        wasm_func_addr_t funcaddr = wrun_store_alloc_func(store, winst, func);
        vec_push_back(&winst->funcaddrs, sizeof(wasm_func_addr_t), &funcaddr);
    }
}

wasm_table_addr_t wrun_store_alloc_table(WasmStore *store, WasmTable *table, WasmRefValue initval) {
    WasmTableInst tinst;
    tinst.tabletype = *table;
    vec_init_with_size(&tinst.elems, sizeof(WasmRefValue), table->limits.min, &initval);
    return vec_push_back(&store->tables, sizeof(WasmTableInst), &tinst) + 1;
}

void wrun_store_alloc_tables(WasmStore *store, VEC(wasm_table_addr_t) *tableaddrs ,VEC(WasmTable) *tables) {
    for (size_t i = 0; i < tables->len; i++) {
        WasmTable *table = tables->ptr + (i * sizeof(WasmTable));
        wasm_table_addr_t tableaddr = wrun_store_alloc_table(store, table, 0);
        vec_push_back(tableaddrs, sizeof(wasm_table_addr_t), &tableaddr);
    }
}

wasm_mem_addr_t wrun_store_alloc_mem(WasmStore *store, WasmMemType *mem) {
    WasmMemInst minst;
    minst.memtype = *mem;
    vec_init_with_zeros(&minst.data, 1, WMEM_PAGE_SIZE * mem->limits.min);
    return vec_push_back(&store->mems, sizeof(WasmMemInst), &minst) + 1;
}

void wrun_store_alloc_mems(WasmStore *store, VEC(wasm_mem_addr_t) *memaddrs, VEC(WasmMemType) *mems) {
    for (size_t i = 0; i < mems->len; i++) {
        WasmMemType *mem = mems->ptr + (i * sizeof(WasmMemType));
        wasm_mem_addr_t memaddr = wrun_store_alloc_mem(store, mem);
        vec_push_back(memaddrs, sizeof(wasm_mem_addr_t), &memaddr);
    }
}

wasm_global_addr_t wrun_store_alloc_global(WasmStore *store, WasmGlobalType *globaltype, WasmValue val) {
    WasmGlobalInst ginst;
    ginst.globaltype = *globaltype;
    ginst.val = val;
    return vec_push_back(&store->globals, sizeof(WasmGlobalInst), &ginst) + 1;
}

void wrun_store_alloc_globals(WasmStore *store, VEC(wasm_global_addr_t) *globaladdrs, VEC(WasmGlobalType) *globals, VEC(WasmValue) *globalinit) {
    for (size_t i = 0; i < globals->len; i++) {
        WasmGlobal *global = globals->ptr + (i * sizeof(WasmGlobal));
        WasmValue initval = *(WasmValue *)vec_at(globalinit, sizeof(WasmValue), i);
        wasm_global_addr_t globaladdr = wrun_store_alloc_global(store, &global->globaltype, initval);
        vec_push_back(globaladdrs, sizeof(wasm_global_addr_t), &globaladdr);
    }
}

wasm_elem_addr_t wrun_store_alloc_elem(WasmStore *store, WasmElem *elem, VEC(WasmRefValue) *references) {
    WasmElemInst einst;
    vec_init(&einst.elem);
    vec_clone(references, &einst.elem, sizeof(WasmRefValue));
    einst.reftype = elem->reftype;
    return vec_push_back(&store->elems, sizeof(WasmElemInst), &einst) + 1;
}

void wrun_store_alloc_elems(WasmStore *store, VEC(wasm_elem_addr_t) *elemaddrs, VEC(WasmElem) *elems, VEC(VEC(WasmRefValue)) *references) {
    for (size_t i = 0; i < elems->len; i++) {
        WasmElem *elem = elems->ptr + (i * sizeof(WasmElem));
        Vec *references = vec_at(references, sizeof(Vec), i);
        wasm_elem_addr_t elemaddr = wrun_store_alloc_elem(store, elem, references);
        vec_push_back(elemaddrs, sizeof(wasm_elem_addr_t), &elemaddr);
    }
}

wasm_data_addr_t wrun_store_alloc_data(WasmStore *store, WasmData *wdata) {
    WasmDataInst dinst;
    dinst.bytes= wdata->bytes;
    return vec_push_back(&store->datas, sizeof(WasmDataInst), &dinst) + 1;
}

void wrun_store_alloc_datas(WasmStore *store, VEC(wasm_data_addr_t) *dataaddrs, VEC(WasmData) *wdatas) {
    for (size_t i = 0; i < wdatas->len; i++) {
        WasmData *wdata = wdatas->ptr + (i * sizeof(WasmData));
        wasm_data_addr_t dataaddr = wrun_store_alloc_data(store, wdata);
        vec_push_back(dataaddrs, sizeof(wasm_data_addr_t), &dataaddr);
    }
}

void wrun_init_params_init(WasmInitParams *params, VEC(WasmExternVal) *imports) {
    vec_init(&params->globalinit);
    vec_init(&params->references);
    params->imports = *imports;
}

void wrun_apply_imports(VEC(WasmExternVal) *imports, WasmModuleInst *winst) {
    for (size_t i = 0; i < imports->len; i++) {
        WasmExternVal *import = vec_at(imports, sizeof(WasmExternVal), i);
        switch (import->kind) {
            case WasmExternValFunc:
                vec_push_back(&winst->funcaddrs, sizeof(wasm_func_addr_t), &import->val.func);
                break;
            case WasmExternValMem:
                vec_push_back(&winst->memaddrs, sizeof(wasm_mem_addr_t), &import->val.mem);
                break;
            case WasmExternValGlobal:
                vec_push_back(&winst->globaladdrs, sizeof(wasm_global_addr_t), &import->val.global);
                break;
            case WasmExternValTable:
                vec_push_back(&winst->tableaddrs, sizeof(wasm_table_addr_t), &import->val.table);
                break;
        }
    }
}

void wrun_instance_assign_exports(VEC(WasmExport) *exports, WasmModuleInst *winst) {
    for (size_t i = 0; i < exports->len; i++) {
        WasmExport *wexp = exports->ptr + (i * sizeof(WasmExport));
        WasmExportInst inst;
        inst.name = wexp->name;
        switch (wexp->desc.kind) {
            case WasmExportMem: {
                inst.val.kind = WasmExternValMem;
                wasm_mem_idx_t idx = wexp->desc.value.mem;
                inst.val.val.mem = *(wasm_mem_addr_t*)vec_at(&winst->memaddrs, sizeof(wasm_mem_addr_t), idx);
                break;
            }
            case WasmExportFunc: {
                inst.val.kind = WasmExternValFunc;
                wasm_func_idx_t idx = wexp->desc.value.func;
                inst.val.val.func = *(wasm_func_addr_t*)vec_at(&winst->funcaddrs, sizeof(wasm_func_addr_t), idx);
                break;
            }
            case WasmExportTable: {
                inst.val.kind = WasmExternValTable;
                wasm_table_idx_t idx = wexp->desc.value.table;
                inst.val.val.table = *(wasm_table_addr_t*)vec_at(&winst->tableaddrs, sizeof(wasm_table_addr_t), idx);
                break;
            }
            case WasmExportGlobal: {
                inst.val.kind = WasmExternValGlobal;
                wasm_global_idx_t idx = wexp->desc.value.global;
                inst.val.val.global = *(wasm_global_addr_t*)vec_at(&winst->globaladdrs, sizeof(wasm_global_addr_t), idx);
                break;
            }
        }
        vec_push_back(&winst->exports, sizeof(WasmExportInst), &inst);
    }
}

void winst_init(WasmModuleInst *winst) {
    winst->types = NULL;
    vec_init(&winst->funcaddrs);
    vec_init(&winst->tableaddrs);
    vec_init(&winst->memaddrs);
    vec_init(&winst->globaladdrs);
    vec_init(&winst->elemaddrs);
    vec_init(&winst->dataaddrs);
    vec_init(&winst->exports);
}

WasmModuleInst *wrun_store_alloc_module(WasmStore *store, WasmModule *wmod, WasmInitParams *params) {
    assert(params->globalinit.len == wmod->globals.len);
    assert(params->imports.len == wmod->imports.len);
    assert(params->references.len == wmod->elems.len);

    WasmModuleInst *winst = malloc(sizeof(WasmModuleInst));
    winst_init(winst);

    winst->types = wmod->types.ptr;
    wrun_apply_imports(&params->imports, winst);
    wrun_store_alloc_funcs(store, winst, &wmod->funcs);
    wrun_store_alloc_tables(store, &winst->tableaddrs, &wmod->tables);
    wrun_store_alloc_mems(store, &winst->memaddrs, &wmod->mems);
    wrun_store_alloc_globals(store, &winst->globaladdrs, &wmod->globals, &params->globalinit);
    wrun_store_alloc_elems(store, &winst->elemaddrs, &wmod->elems, &params->references);
    wrun_store_alloc_datas(store, &winst->dataaddrs, &wmod->datas);
    wrun_instance_assign_exports(&wmod->exports, winst);

    return winst;
}

WasmModuleInst *wrun_alloc_auxiliary_module(WasmModule *wmod, WasmStore *store, VEC(WasmExternVal) *imports) {
    assert(wmod->imports.len == imports->len);

    WasmModuleInst *winst = malloc(sizeof(WasmModuleInst));
    winst_init(winst);

    winst->types = wmod->types.ptr;

    for (size_t i = 0; i < imports->len; i++) {
        WasmExternVal *import = vec_at(imports, sizeof(WasmExternVal), i);
        switch (import->kind) {
            case WasmExternValFunc:
                vec_push_back(&winst->funcaddrs, sizeof(wasm_func_addr_t), &import->val.func);
                break;
            case WasmExternValGlobal:
                vec_push_back(&winst->globaladdrs, sizeof(wasm_global_addr_t), &import->val.global);
                break;
            default:
                break;
        }
    }

    wrun_store_alloc_funcs(store, winst, &wmod->funcs);

    return winst;
}

void wrun_dump_init_params(WasmInitParams *params) {
    printf("globalinit: [");
    for (size_t i = 0; i < params->globalinit.len; i++) {
        WasmValue *val = vec_at(&params->globalinit, sizeof(WasmValue), i);
        printf("%d, ", val->num.i32);
    }
    printf("],\n");
}

WasmModuleInst *wrun_instantiate_module(WasmModule *wmod, WasmStore *store, VEC(WasmExternVal) *imports) {
    WasmInitParams params;
    wrun_init_params_init(&params, imports);

    WasmStack stack;
    wrun_stack_init(&stack);
    WasmModuleInst *winst_init = wrun_alloc_auxiliary_module(wmod, store, imports);
    wrun_stack_push_auxiliary_frame(&stack, winst_init);

    for (size_t i = 0; i < wmod->globals.len; i++) {
        WasmGlobal* global = vec_at(&wmod->globals, sizeof(WasmGlobal), i);
        WasmValue out;
        WasmResultKind res = wrun_eval_const_expr(store, &stack, global->init.ptr, &out);
        vec_push_back(&params.globalinit, sizeof(WasmValue), &out);
    }

    // eval element segments

    wrun_dump_init_params(&params);
    return wrun_store_alloc_module(store, wmod, &params);
}

void wrun_stack_init(WasmStack *stack) {
    vec_init(&stack->entries);
}

size_t wrun_stack_push(WasmStack *stack, WasmStackEntry *entry) {
    return vec_push_back(&stack->entries, sizeof(WasmStackEntry), entry);
}

size_t wrun_stack_push_val(WasmStack *stack, WasmValue *val) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryValue,
        .entry.val = *val
    };
    return wrun_stack_push(stack, &entry);
}

size_t wrun_stack_push_i32(WasmStack *stack, int32_t val) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryValue,
        .entry.val.num.i32 = val
    };
    return wrun_stack_push(stack, &entry);
}

size_t wrun_stack_push_i64(WasmStack *stack, int64_t val) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryValue,
        .entry.val.num.i64 = val
    };
    return wrun_stack_push(stack, &entry);
}

size_t wrun_stack_push_f32(WasmStack *stack, float val) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryValue,
        .entry.val.num.f32 = val
    };
    return wrun_stack_push(stack, &entry);
}

size_t wrun_stack_push_f64(WasmStack *stack, double val) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryValue,
        .entry.val.num.f64 = val
    };
    return wrun_stack_push(stack, &entry);
}

size_t wrun_stack_push_ref(WasmStack *stack, wasm_addr_t ref) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryValue,
        .entry.val.ref = ref
    };
    return wrun_stack_push(stack, &entry);
}

size_t wrun_stack_push_auxiliary_frame(WasmStack *stack, WasmModuleInst *winst) {
    WasmStackEntry frame;
    frame.kind = WasmStackEntryActivation;
    frame.entry.activation.return_arity = 0;
    frame.entry.activation.inst = winst;
    vec_init(&frame.entry.activation.locals);
    return wrun_stack_push(stack, &frame);
}

WasmActivation *wrun_stack_find_current_frame(WasmStack *stack) {
    for (size_t i = stack->entries.len; i >= 0; i--) {
        WasmStackEntry *entry = vec_at(&stack->entries, sizeof(WasmStackEntry), i);
        if (entry->kind == WasmStackEntryActivation) {
            return &entry->entry.activation;
        }
    }
    assert(false); // unreachable: no activation frame!
}

bool wrun_stack_pop(WasmStack *stack, WasmStackEntry *out) {
    return vec_pop_back(&stack->entries, sizeof(WasmStackEntry), out);
}

bool wrun_stack_pop_val(WasmStack *stack, WasmValue *out) {
    WasmStackEntry entry;
    bool popped = wrun_stack_pop(stack, &entry);
    *out = entry.entry.val;
    return popped;
}

WasmResultKind wrun_eval_const_expr(WasmStore *store, WasmStack *stack, WasmInstruction *expr, WasmValue *wval) {
    WasmInstruction* ip = expr;
    while (true) {
        switch (ip->opcode) {
            case WasmOpI32Const:
                wrun_stack_push_i32(stack, ip->params._const.value.i32);
                break;
            case WasmOpI64Const:
                wrun_stack_push_i64(stack, ip->params._const.value.i64);
                break;
            case WasmOpF32Const:
                wrun_stack_push_f32(stack, ip->params._const.value.f32);
                break;
            case WasmOpF64Const:
                wrun_stack_push_f64(stack, ip->params._const.value.f64);
                break;
            case WasmOpRefNull:
                wrun_stack_push_ref(stack, 0);
                break;
            case WasmOpRefFunc: {
                wasm_func_idx_t funcidx = ip->params.ref_func.funcidx;
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                wasm_func_addr_t *funcaddr = vec_at(&frame->inst->funcaddrs, sizeof(wasm_addr_t), funcidx - 1);
                wrun_stack_push_ref(stack, *funcaddr);
                break;
            }
            case WasmOpGlobalGet: {
                wasm_global_idx_t globalidx = ip->params.var.idx.global;
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                wasm_global_addr_t globaladdr = *(wasm_global_addr_t*)vec_at(&frame->inst->globaladdrs, sizeof(wasm_addr_t), globalidx - 1);
                WasmGlobalInst *glob = (WasmGlobalInst*)vec_at(&store->globals, sizeof(WasmGlobalInst), globaladdr);
                wrun_stack_push_val(stack, &glob->val);
                break;
            }
            case WasmOpExprEnd:
                wrun_stack_pop_val(stack, wval);
                return Ok;
            default:
                printf("unhandled opcode [%s]\n", wmod_str_opcode(ip->opcode));
                return Trap;
        }
        ip++;
    }
}
