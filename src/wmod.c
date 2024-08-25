
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
    vec_init(&wmod->imports);
    vec_init(&wmod->exports);
    vec_init(&wmod->customs);
    wmod->meta.version = 0;
    wmod->start.present = false;
    wmod->start.func_idx = 0;
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
        printf("<t%u> ", data[i].type_idx);
        printf("locals(%zu) ", data[i].locals.len);
        printf("body(%zu)\n", data[i].body.len);
    }
}

void wmod_dump_limits(WasmLimits *limits) {
    printf("[%d", limits->min);
    if (limits->bounded) {
        printf(", %d", limits->max);
    }
    printf("]");
}

void wmod_dump_table(WasmTable *table) {
    wmod_dump_limits(&table->limits);
    printf(" %s\n", wmod_str_ref_type(table->reftype));
}

void wmod_dump_tables(WasmTables *tables) {
    WasmTable *data = tables->ptr;
    for (size_t i = 0; i < tables->len; i++) {
        printf("<tb%zu>: ", i);
        wmod_dump_table(&data[i]);
    }
}

void wmod_dump_mems(WasmMems *mems) {
    WasmMemType *data = mems->ptr;
    for (size_t i = 0; i < mems->len; i++) {
        printf("<m%zu>: ", i);
        wmod_dump_limits(&data[i].limits);
        printf("\n");
    }
}

void wmod_dump_name(WasmName *name) {
    fwrite(name->bytes, 1, name->len, stdout);
}

void wmod_dump_global_mutability(WasmGlobalMutability mut) {
    switch (mut) {
        case WasmGlobalConst:
            printf("const");
            break;
        case WasmGlobalVar:
            printf("var");
            break;
    }
}

void wmod_dump_import_desc(WasmImportDesc *desc) {
    switch (desc->kind) {
        case WasmImportFunc:
            printf("func <t%u>", desc->value.func);
            break;
        case WasmImportTable:
            printf("table ");
            wmod_dump_table(&desc->value.table);
            break;
        case WasmImportMem:
            printf("mem ");
            wmod_dump_limits(&desc->value.mem.limits);
            break;
        case WasmImportGlobal:
            printf("global ");
            wmod_dump_global_mutability(desc->value.global.mut);
            printf(" ");
            wmod_dump_val_type(&desc->value.global.valtype);
            break;
    }
}

void wmod_dump_import(WasmImport *import) {
    wmod_dump_name(&import->module_name);
    printf("::");
    wmod_dump_name(&import->item_name);
    printf(" ");
    wmod_dump_import_desc(&import->desc);
}

void wmod_dump_imports(WasmImports *imports) {
    WasmImport *data = imports->ptr;
    for (size_t i = 0; i < imports->len; i++) {
        wmod_dump_import(&data[i]);
        printf("\n");
    }
}

void wmod_dump_export_desc(WasmExportDesc *desc) {
    switch (desc->kind) {
        case WasmExportFunc:
            printf("func <f%u>", desc->value.func);
            break;
        case WasmExportTable:
            printf("table <tb%u>", desc->value.table);
            break;
        case WasmExportMem:
            printf("mem <m%u>", desc->value.mem);
            break;
        case WasmExportGlobal:
            printf("global <g%u>", desc->value.global);
            break;
    }
}

void wmod_dump_export(WasmExport *exp) {
    wmod_dump_name(&exp->name);
    printf(" ");
    wmod_dump_export_desc(&exp->desc);
}

void wmod_dump_exports(WasmExports *exp) {
    WasmExport *data = exp->ptr;
    for (size_t i = 0; i < exp->len; i++) {
        wmod_dump_export(&data[i]);
        printf("\n");
    }
}

void wmod_dump_start(WasmStart *start) {
    if (start->present) {
        printf("start: <f%u>\n", start->func_idx);
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
    printf("-------mems: %zu-------\n", wmod->mems.len);
    wmod_dump_mems(&wmod->mems);
    printf("-------imports: %zu-------\n", wmod->imports.len);
    wmod_dump_imports(&wmod->imports);
    printf("-------exports: %zu-------\n", wmod->exports.len);
    wmod_dump_exports(&wmod->exports);
    printf("-------start: %d-------\n", wmod->start.present);
    wmod_dump_start(&wmod->start);
}

void wmod_name_init(WasmName *name) {
    name->len = 0;
    name->bytes = NULL;
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

void wmod_import_init(WasmImport *import) {
    wmod_name_init(&import->module_name);
    wmod_name_init(&import->item_name);
}

void wmod_export_init(WasmExport *exp) {
    wmod_name_init(&exp->name);
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

wasm_mem_idx_t wmod_push_back_mem(WasmModule *wmod, WasmMemType *mem) {
    return vec_push_back(&wmod->mems, sizeof(WasmMemType), mem);
}

void wmod_push_back_import(WasmModule *wmod, WasmImport *import) {
    vec_push_back(&wmod->imports, sizeof(WasmImport), import);
}

void wmod_push_back_export(WasmModule *wmod, WasmExport *exp) {
    vec_push_back(&wmod->exports, sizeof(WasmExport), exp);
}

void wmod_func_push_back_locals(WasmFunc *func, u_int32_t n, WasmValueType *valtype) {
    while (n > 0) {
        vec_push_back(&func->locals, sizeof(WasmValueType), valtype);
        n--;
    }
}

void wmod_func_push_back_instruction(WasmFunc *func, WasmInstruction *ins) {
    vec_push_back(&func->body, sizeof(WasmInstruction), ins);
}
