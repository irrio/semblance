
#include "wrun.h"
#include "vec.h"
#include "wmod.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <string.h>

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
    dinst.len = wdata->len;
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
        WasmResultKind res = wrun_eval_expr(store, &stack, global->init.ptr, &out);
        vec_push_back(&params.globalinit, sizeof(WasmValue), &out);
    }

    for (size_t i = 0; i < wmod->elems.len; i++) {
        WasmElem *elem = vec_at(&wmod->elems, sizeof(WasmElem), i);
        WasmValue out;
        WasmResultKind res = wrun_eval_expr(store, &stack, elem->init.ptr, &out);
        vec_push_back(&params.references, sizeof(WasmValue), &out);
    }

    wrun_stack_pop_and_drop(&stack);
    WasmModuleInst *winst = wrun_store_alloc_module(store, wmod, &params);
    wrun_stack_push_auxiliary_frame(&stack, winst);

    WasmInstruction progbuf[5];
    for (size_t i = 0; i < wmod->elems.len; i++) {
        WasmElem *elem = vec_at(&wmod->elems, sizeof(WasmElem), i);
        switch (elem->elemmode.kind) {
            case WasmElemModeActive: {
                size_t n = elem->init.len;
                wrun_exec_expr(store, &stack, elem->elemmode.value.active.offset_expr.ptr);
                progbuf[0].opcode = WasmOpI32Const;
                progbuf[0].params._const.value.i32 = 0;
                progbuf[1].opcode = WasmOpI32Const;
                progbuf[1].params._const.value.i32 = n;
                progbuf[2].opcode = WasmOpTableInit;
                progbuf[2].params.table_init.tableidx = elem->elemmode.value.active.tableidx;
                progbuf[2].params.table_init.elemidx = i;
                progbuf[3].opcode = WasmOpElemDrop;
                progbuf[3].params.elem_drop.elemidx = i;
                progbuf[4].opcode = WasmOpExprEnd;
                wrun_exec_expr(store, &stack, progbuf);
                break;
            }
            case WasmElemModeDeclarative:
                progbuf[0].opcode = WasmOpElemDrop;
                progbuf[0].params.elem_drop.elemidx = i;
                progbuf[1].opcode = WasmOpExprEnd;
                wrun_exec_expr(store, &stack, progbuf);
                break;
            default:
                continue;
        }
    }

    for (size_t i = 0; i < wmod->datas.len; i++) {
        WasmData *wdata = vec_at(&wmod->datas, sizeof(WasmData), i);
        switch (wdata->datamode.kind) {
            case WasmDataModeActive: {
                assert(wdata->datamode.value.active.memidx == 0);
                u_int32_t n = wdata->len;
                wrun_exec_expr(store, &stack, wdata->datamode.value.active.offset_expr.ptr);
                progbuf[0].opcode = WasmOpI32Const;
                progbuf[0].params._const.value.i32 = 0;
                progbuf[1].opcode = WasmOpI32Const;
                progbuf[1].params._const.value.i32 = n;
                progbuf[2].opcode = WasmOpMemoryInit;
                progbuf[2].params.mem_init.dataidx = i;
                progbuf[3].opcode = WasmOpDataDrop;
                progbuf[3].params.mem_init.dataidx = i;
                progbuf[4].opcode = WasmOpExprEnd;
                WasmResultKind res = wrun_exec_expr(store, &stack, progbuf);
                if (res == Trap) {
                    printf("TRAP!\n");
                }
                break;
            }
            default:
                continue;
        }
    }

    if (wmod->start.present) {
        progbuf[0].opcode = WasmOpCall;
        progbuf[0].params.call.funcidx = wmod->start.func_idx;
        progbuf[1].opcode = WasmOpExprEnd;
        wrun_exec_expr(store, &stack, progbuf);
    }

    wrun_stack_pop_and_drop(&stack);

    return winst;
}

void wrun_stack_init(WasmStack *stack) {
    vec_init(&stack->entries);
}

size_t wrun_stack_push(WasmStack *stack, WasmStackEntry *entry) {
    return vec_push_back(&stack->entries, sizeof(WasmStackEntry), entry);
}

size_t wrun_stack_push_label(WasmStack *stack, WasmLabel *label) {
    WasmStackEntry entry = {
        .kind = WasmStackEntryLabel,
        .entry.label = *label
    };
    return wrun_stack_push(stack, &entry);
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

size_t wrun_stack_push_frame(WasmStack *stack, WasmModuleInst *winst, VEC(WasmValue) *locals, u_int32_t arity) {
    WasmStackEntry frame;
    frame.kind = WasmStackEntryActivation;
    frame.entry.activation.inst = winst;
    frame.entry.activation.return_arity = arity;
    frame.entry.activation.locals = *locals;
    return wrun_stack_push(stack, &frame);
}

size_t wrun_stack_push_auxiliary_frame(WasmStack *stack, WasmModuleInst *winst) {
    VEC(WasmValue) locals;
    vec_init(&locals);
    return wrun_stack_push_frame(stack, winst, &locals, 0);
}

size_t wrun_stack_push_dummy_frame(WasmStack *stack) {
    WasmModuleInst *winst = malloc(sizeof(WasmModuleInst));
    winst_init(winst);
    VEC(WasmValue) locals;
    vec_init(&locals);
    return wrun_stack_push_frame(stack, winst, &locals, 0);
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

bool wrun_stack_pop_and_drop(WasmStack *stack) {
    return vec_pop_back_and_drop(&stack->entries);
}

bool wrun_stack_pop_val(WasmStack *stack, WasmValue *out) {
    WasmStackEntry entry;
    bool popped = wrun_stack_pop(stack, &entry);
    *out = entry.entry.val;
    return popped;
}

WasmResultKind wrun_eval_expr(WasmStore *store, WasmStack *stack, WasmInstruction *expr, WasmValue *wval) {
    WasmResultKind res = wrun_exec_expr(store, stack, expr);
    if (res == Ok) {
        wrun_stack_pop_val(stack, wval);
    }
    return res;
}

WasmResultKind wrun_exec_expr(WasmStore *store, WasmStack *stack, WasmInstruction *expr) {
    WasmInstruction* ip = expr;
    while (true) {
        printf("EXEC OPCODE: %s\n", wmod_str_opcode(ip->opcode));
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
                wasm_func_addr_t *funcaddr = vec_at(&frame->inst->funcaddrs, sizeof(wasm_addr_t), funcidx);
                wrun_stack_push_ref(stack, *funcaddr);
                break;
            }
            case WasmOpGlobalGet: {
                wasm_global_idx_t globalidx = ip->params.var.idx.global;
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                wasm_global_addr_t globaladdr = *(wasm_global_addr_t*)vec_at(&frame->inst->globaladdrs, sizeof(wasm_addr_t), globalidx);
                WasmGlobalInst *glob = (WasmGlobalInst*)vec_at(&store->globals, sizeof(WasmGlobalInst), globaladdr - 1);
                wrun_stack_push_val(stack, &glob->val);
                break;
            }
            case WasmOpLocalSet: {
                wasm_local_idx_t localidx = ip->params.var.idx.local;
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                WasmValue *local = vec_at(&frame->locals, sizeof(WasmValue), localidx);
                wrun_stack_pop_val(stack, local);
                break;
            }
            case WasmOpLocalGet: {
                wasm_local_idx_t localidx = ip->params.var.idx.local;
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                WasmValue *local = vec_at(&frame->locals, sizeof(WasmValue), localidx);
                wrun_stack_push_val(stack, local);
                break;
            }
            case WasmOpMemoryInit: {
                wasm_data_idx_t x = ip->params.mem_init.dataidx;
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                wasm_mem_addr_t ma = *(wasm_mem_addr_t*)frame->inst->memaddrs.ptr;
                WasmMemInst *mem = vec_at(&store->mems, sizeof(WasmMemInst), ma - 1);
                wasm_data_addr_t da = *(wasm_data_addr_t*)vec_at(&frame->inst->dataaddrs, sizeof(wasm_data_addr_t), x);
                WasmDataInst *data = vec_at(&store->datas, sizeof(WasmDataInst), da - 1);
                WasmValue n;
                wrun_stack_pop_val(stack, &n);
                WasmValue s;
                wrun_stack_pop_val(stack, &s);
                WasmValue d;
                wrun_stack_pop_val(stack, &d);
                if (s.num.i32 + n.num.i32 > data->len) {
                    return Trap;
                }
                if (d.num.i32 + n.num.i32 > mem->data.len) {
                    return Trap;
                }
                if (n.num.i32 == 0) {
                    break;
                }
                memcpy((mem->data.ptr + d.num.i32), (data->bytes + s.num.i32), n.num.i32);
                break;
            }
            case WasmOpDataDrop: {
                WasmActivation *frame = wrun_stack_find_current_frame(stack);
                wasm_data_idx_t dataidx = ip->params.mem_init.dataidx;
                wasm_data_addr_t *dataaddr = vec_at(&frame->inst->dataaddrs, sizeof(wasm_data_addr_t), dataidx);
                WasmDataInst *wdata = vec_at(&store->datas, sizeof(WasmDataInst), *dataaddr - 1);
                wdata->bytes = NULL;
                wdata->len = 0;
                break;
            }
            case WasmOpExprEnd:
                return Ok;
            default:
                printf("unhandled opcode [%s]\n", wmod_str_opcode(ip->opcode));
                return Trap;
        }
        ip++;
    }
}

WasmExternVal wrun_resolve_export(WasmModuleInst *winst, char *name) {
    size_t slen = strlen(name);
    for (size_t i = 0; i < winst->exports.len; i++) {
        WasmExportInst *wexp = vec_at(&winst->exports, sizeof(WasmExportInst), i);
        if (slen == wexp->name.len && memcmp(wexp->name.bytes, name, slen) == 0) {
            return wexp->val;
        }
    }
    assert(false); // export not found
}

WasmResult wrun_invoke_func(WasmModuleInst *winst, wasm_func_addr_t funcaddr, VEC(WasmValue) *args, WasmStore *store) {
    WasmResult out;
    out.kind = Ok;
    vec_init(&out.values);

    WasmStack stack;
    wrun_stack_init(&stack);
    wrun_stack_push_dummy_frame(&stack);

    VEC(WasmValue) locals;
    vec_init(&locals);

    WasmFuncInst *finst = vec_at(&store->funcs, sizeof(WasmFuncInst), funcaddr - 1);

    for (size_t i = 0; i < args->len; i++) {
        WasmValue *local = vec_at(args, sizeof(WasmValue), i);
        vec_push_back(&locals, sizeof(WasmValue), local);
    }
    for (size_t i = 0; i < finst->val.wasmfunc.func->locals.len; i++) {
        WasmValue local;
        WasmValueType *localtype = vec_at(&finst->val.wasmfunc.func->locals, sizeof(WasmValueType), i);
        wrun_value_default(*localtype, &local);
        vec_push_back(&locals, sizeof(WasmValue), &local);
    }
    uint32_t out_arity = finst->functype.output_type.len;
    wrun_stack_push_frame(&stack, finst->val.wasmfunc.module, &locals, out_arity);
    WasmLabel label = {
        .argument_arity = out_arity,
        .instr = NULL
    };
    wrun_stack_push_label(&stack, &label);

    out.kind = wrun_exec_expr(store, &stack, finst->val.wasmfunc.func->body.ptr);
    for (size_t i = 0; i < out_arity; i++) {
        WasmValue val;
        wrun_stack_pop_val(&stack, &val);
        vec_push_back(&out.values, sizeof(WasmValue), &val);
    }

    return out;
}
