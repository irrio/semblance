
#include "wmod.h"
#include "vec.h"
#include <stdio.h>

void wmod_init(WasmModule *wmod) {
    vec_init(&wmod->types);
    vec_init(&wmod->funcs);
    vec_init(&wmod->tables);
    vec_init(&wmod->mems);
    vec_init(&wmod->globals);
    vec_init(&wmod->elems);
    vec_init(&wmod->datas);
    vec_init(&wmod->start);
    vec_init(&wmod->imports);
    vec_init(&wmod->exports);
    vec_init(&wmod->customs);
    wmod->meta.version = 0;
}

char *wmod_str_num_type(WasmNumType numtype) {
    switch (numtype) {
        case WasmNumI32:
            return "i32";
        case WasmNumI64:
            return "i64";
        case WasmNumF32:
            return "f32";
        case WasmNumF64:
            return "f64";
    }
}

char *wmod_str_ref_type(WasmRefType reftype) {
    switch (reftype) {
        case WasmRefExtern:
            return "externref";
        case WasmRefFunc:
            return "funcref";
    }
}

char *wmod_str_vec_type(WasmVecType vectype) {
    switch (vectype) {
        case WasmVecV128:
            return "v128";
    }
}

void wmod_dump_val_type(WasmValueType *valtype) {
    switch (valtype->kind) {
        case WasmValueTypeNum:
            printf("%s", wmod_str_num_type(valtype->value.num));
            break;
        case WasmValueTypeRef:
            printf("%s", wmod_str_ref_type(valtype->value.ref));
            break;
        case WasmValueTypeVec:
            printf("%s", wmod_str_vec_type(valtype->value.vec));
            break;
    }
}

void wmod_dump_result_type(WasmResultType *resulttype) {
    WasmValueType *data = resulttype->ptr;
    printf("(");
    for (size_t i = 0; i < resulttype->len; i++) {
        wmod_dump_val_type(&data[i]);
        if (i != resulttype->len - 1) printf(", ");
    }
    printf(")");
}

void wmod_dump_types(WasmTypes *types) {
    WasmFuncType *data = types->ptr;
    for (size_t i = 0; i < types->len; i++) {
        printf("<t%zu>: ", i);
        wmod_dump_result_type(&data[i].input_type);
        printf(" -> ");
        wmod_dump_result_type(&data[i].output_type);
        printf("\n");
    }
}

void wmod_dump_funcs(WasmFuncs *funcs) {
    WasmFunc *data = funcs->ptr;
    for (size_t i = 0; i < funcs->len; i++) {
        printf("<f%zu>: ", i);
        printf("<t%u>\n", data[i].type_idx);
    }
}

void wmod_dump_limits(WasmLimits *limits) {
    printf("[%d", limits->min);
    if (limits->bounded) {
        printf(", %d", limits->max);
    }
    printf("]");
}

void wmod_dump_tables(WasmTables *tables) {
    WasmTable *data = tables->ptr;
    for (size_t i = 0; i < tables->len; i++) {
        printf("<tb%zu>: ", i);
        wmod_dump_limits(&data[i].limits);
        printf(" %s\n", wmod_str_ref_type(data[i].reftype));
    }
}

void wmod_dump(WasmModule *wmod) {
    printf("version: %d\n", wmod->meta.version);
    printf("-------types: %zu-------\n", wmod->types.len);
    wmod_dump_types(&wmod->types);
    printf("-------funcs: %zu-------\n", wmod->funcs.len);
    wmod_dump_funcs(&wmod->funcs);
    printf("-------tables: %zu-------\n", wmod->tables.len);
    wmod_dump_tables(&wmod->tables);
}

void wmod_func_type_init(WasmFuncType *type) {
    vec_init(&type->input_type);
    vec_init(&type->output_type);
}

void wmod_func_init(WasmFunc *func) {
    func->type_idx = 0;
    vec_init(&func->locals);
    vec_init(&func->body);
}

size_t wmod_result_type_push_back(WasmResultType *type, WasmValueType *valtype) {
    return vec_push_back(type, sizeof(WasmValueType), valtype);
}

wasm_type_idx_t wmod_push_back_type(WasmModule *wmod, WasmFuncType *type) {
    return vec_push_back(&wmod->types, sizeof(WasmFuncType), type);
}

wasm_func_idx_t wmod_push_back_func(WasmModule *wmod, WasmFunc *func) {
    return vec_push_back(&wmod->funcs, sizeof(WasmFunc), func);
}

wasm_table_idx_t wmod_push_back_table(WasmModule *wmod, WasmTable *table) {
    return vec_push_back(&wmod->tables, sizeof(WasmTable), table);
}
