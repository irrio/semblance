
#include "wmod.h"
#include "vec.h"
#include <stdio.h>
#include <string.h>

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
    wmod->meta.datacount = 0;
    wmod->start.present = false;
    wmod->start.func_idx = 0;
}

char *wmod_str_opcode(WasmOpcode opcode) {
    switch (opcode) {
        case WasmOpUnreachable: return "unreachable";
        case WasmOpNop: return "nop";
        case WasmOpBlock: return "block";
        case WasmOpLoop: return "loop";
        case WasmOpIf: return "if";
        case WasmOpElse: return "else";
        case WasmOpBreak: return "break";
        case WasmOpBreakIf: return "br_if";
        case WasmOpBreakTable: return "br_table";
        case WasmOpReturn: return "return";
        case WasmOpCall: return "call";
        case WasmOpCallIndirect: return "call_indirect";
        case WasmOpExprEnd: return "expr_end";
        case WasmOpRefNull: return "ref_null";
        case WasmOpRefIsNull: return "ref_is_null";
        case WasmOpRefFunc: return "ref_func";
        case WasmOpDrop: return "drop";
        case WasmOpSelect: return "select";
        case WasmOpLocalGet: return "local_get";
        case WasmOpLocalSet: return "local_set";
        case WasmOpLocalTee: return "local_tee";
        case WasmOpGlobalGet: return "global_get";
        case WasmOpGlobalSet: return "global_set";
        case WasmOpTableGet: return "table_get";
        case WasmOpTableSet: return "table_set";
        case WasmOpTableSize: return "table_size";
        case WasmOpTableGrow: return "table_grow";
        case WasmOpTableFill: return "table_fill";
        case WasmOpTableCopy: return "table_copy";
        case WasmOpTableInit: return "table_init";
        case WasmOpElemDrop: return "elem_drop";
        case WasmOpI32Load: return "i32_load";
        case WasmOpI64Load: return "i64_load";
        case WasmOpF32Load: return "f32_load";
        case WasmOpF64Load: return "f64_load";
        case WasmOpI32Load8_s: return "i32_load8_s";
        case WasmOpI32Load8_u: return "i32_load8_u";
        case WasmOpI32Load16_s: return "i32_load16_s";
        case WasmOpI32Load16_u: return "i32_load16_u";
        case WasmOpI64Load8_s: return "i64_load8_s";
        case WasmOpI64Load8_u: return "i64_load8_u";
        case WasmOpI64Load16_s: return "i64_load16_s";
        case WasmOpI64Load16_u: return "i64_load16_u";
        case WasmOpI64Load32_s: return "i64_load32_s";
        case WasmOpI64Load32_u: return "i64_load32_u";
        case WasmOpI32Store: return "i32_store";
        case WasmOpI64Store: return "i64_store";
        case WasmOpF32Store: return "f32_store";
        case WasmOpF64Store: return "f64_store";
        case WasmOpI32Store8: return "i32_store8";
        case WasmOpI32Store16: return "i32_store16";
        case WasmOpI64Store8: return "i64_store8";
        case WasmOpI64Store16: return "i64_store16";
        case WasmOpI64Store32: return "i64_store32";
        case WasmOpMemorySize: return "memory_size";
        case WasmOpMemoryGrow: return "memory_grow";
        case WasmOpMemoryInit: return "memory_init";
        case WasmOpDataDrop: return "data_drop";
        case WasmOpMemoryCopy: return "memory_copy";
        case WasmOpMemoryFill: return "memory_fill";
        case WasmOpI32Const: return "i32_const";
        case WasmOpI64Const: return "i64_const";
        case WasmOpF32Const: return "f32_const";
        case WasmOpF64Const: return "f64_const";
        case WasmOpI32EqZ: return "i32_eqz";
        case WasmOpI32Eq: return "i32_eq";
        case WasmOpI32Neq: return "i32_neq";
        case WasmOpI32Lt_s: return "i32_lt_s";
        case WasmOpI32Lt_u: return "i32_lt_u";
        case WasmOpI32Gt_s: return "i32_gt_s";
        case WasmOpI32Gt_u: return "i32_gt_u";
        case WasmOpI32Le_s: return "i32_le_s";
        case WasmOpI32Le_u: return "i32_le_u";
        case WasmOpI32Ge_s: return "i32_ge_s";
        case WasmOpI32Ge_u: return "i32_ge_u";
        case WasmOpI64EqZ: return "i64_eq_z";
        case WasmOpI64Eq: return "i64_eq";
        case WasmOpI64Neq: return "i64_neq";
        case WasmOpI64Lt_s: return "i64_lt_s";
        case WasmOpI64Lt_u: return "i64_lt_u";
        case WasmOpI64Gt_s: return "i64_gt_s";
        case WasmOpI64Gt_u: return "i64_gt_u";
        case WasmOpI64Le_s: return "i64_le_s";
        case WasmOpI64Le_u: return "i64_le_u";
        case WasmOpI64Ge_s: return "i64_ge_s";
        case WasmOpI64Ge_u: return "i64_ge_u";
        case WasmOpF32Eq: return "f32_eq";
        case WasmOpF32Neq: return "f32_neq";
        case WasmOpF32Lt: return "f32_lt";
        case WasmOpF32Gt: return "f32_gt";
        case WasmOpF32Le: return "f32_le";
        case WasmOpF32Ge: return "f32_ge";
        case WasmOpF64Eq: return "f64_eq";
        case WasmOpF64Neq: return "f64_neq";
        case WasmOpF64Lt: return "f64_lt";
        case WasmOpF64Gt: return "f64_gt";
        case WasmOpF64Le: return "f64_le";
        case WasmOpF64Ge: return "f64_ge";
        case WasmOpI32Clz: return "i32_clz";
        case WasmOpI32Ctz: return "i32_ctz";
        case WasmOpI32Popcnt: return "i32_popcnt";
        case WasmOpI32Add: return "i32_add";
        case WasmOpI32Sub: return "i32_sub";
        case WasmOpI32Mul: return "i32_mul";
        case WasmOpI32Div_s: return "i32_div_s";
        case WasmOpI32Div_u: return "i32_div_u";
        case WasmOpI32Rem_s: return "i32_rem_s";
        case WasmOpI32Rem_u: return "i32_rem_u";
        case WasmOpI32And: return "i32_and";
        case WasmOpI32Or: return "i32_or";
        case WasmOpI32Xor: return "i32_xor";
        case WasmOpI32Shl: return "i32_shl";
        case WasmOpI32Shr_s: return "i32_shr_s";
        case WasmOpI32Shr_u: return "i32_shr_u";
        case WasmOpI32Rotl: return "i32_rotl";
        case WasmOpI32Rotr: return "i32_rotr";
        case WasmOpI64Clz: return "i64_clz";
        case WasmOpI64Ctz: return "i64_ctz";
        case WasmOpI64Popcnt: return "i64_popcnt";
        case WasmOpI64Add: return "i64_add";
        case WasmOpI64Sub: return "i64_sub";
        case WasmOpI64Mul: return "i64_mul";
        case WasmOpI64Div_s: return "i64_div_s";
        case WasmOpI64Div_u: return "i64_div_u";
        case WasmOpI64Rem_s: return "i64_rem_s";
        case WasmOpI64Rem_u: return "i64_rem_u";
        case WasmOpI64And: return "i64_and";
        case WasmOpI64Or: return "i64_or";
        case WasmOpI64Xor: return "i64_xor";
        case WasmOpI64Shl: return "i64_shl";
        case WasmOpI64Shr_s: return "i64_shr_s";
        case WasmOpI64Shr_u: return "i64_shr_u";
        case WasmOpI64Rotl: return "i64_rotl";
        case WasmOpI64Rotr: return "i64_rotr";
        case WasmOpF32Abs: return "f32_abs";
        case WasmOpF32Neg: return "f32_neg";
        case WasmOpF32Ceil: return "f32_ceil";
        case WasmOpF32Floor: return "f32_floor";
        case WasmOpF32Trunc: return "f32_trunc";
        case WasmOpF32Nearest: return "f32_nearest";
        case WasmOpF32Sqrt: return "f32_sqrt";
        case WasmOpF32Add: return "f32_add";
        case WasmOpF32Sub: return "f32_sub";
        case WasmOpF32Mul: return "f32_mul";
        case WasmOpF32Div: return "f32_div";
        case WasmOpF32Min: return "f32_min";
        case WasmOpF32Max: return "f32_max";
        case WasmOpF32CopySign: return "f32_copy_sign";
        case WasmOpF64Abs: return "f64_abs";
        case WasmOpF64Neg: return "f64_neg";
        case WasmOpF64Ceil: return "f64_ceil";
        case WasmOpF64Floor: return "f64_floor";
        case WasmOpF64Trunc: return "f64_trunc";
        case WasmOpF64Nearest: return "f64_nearest";
        case WasmOpF64Sqrt: return "f64_sqrt";
        case WasmOpF64Add: return "f64_add";
        case WasmOpF64Sub: return "f64_sub";
        case WasmOpF64Mul: return "f64_mul";
        case WasmOpF64Div: return "f64_div";
        case WasmOpF64Min: return "f64_min";
        case WasmOpF64Max: return "f64_max";
        case WasmOpF64CopySign: return "f64_copy_sign";
        case WasmOpI32WrapI64: return "i32_wrap_i64";
        case WasmOpI32TruncF32_s: return "i32_trunc_f32_s";
        case WasmOpI32TruncF32_u: return "i32_trunc_f32_u";
        case WasmOpI32TruncF64_s: return "i32_trunc_f64_s";
        case WasmOpI32TruncF64_u: return "i32_trunc_f64_u";
        case WasmOpI64ExtendI32_s: return "i64_extend_i32_s";
        case WasmOpI64ExtendI32_u: return "i64_extend_i32_u";
        case WasmOpI64TruncF32_s: return "i64_trunc_f32_s";
        case WasmOpI64TruncF32_u: return "i64_trunc_f32_u";
        case WasmOpI64TruncF64_s: return "i64_trunc_f64_s";
        case WasmOpI64TruncF64_u: return "i64_trunc_f64_u";
        case WasmOpF32ConvertI32_s: return "f32_convert_i32_s";
        case WasmOpF32ConvertI32_u: return "f32_convert_i32_u";
        case WasmOpF32ConvertI64_s: return "f32_convert_i64_s";
        case WasmOpF32ConvertI64_u: return "f32_convert_i64_u";
        case WasmOpF32DemoteF64: return "f32_demote_f64";
        case WasmOpF64ConvertI32_s: return "f64_convert_i32_s";
        case WasmOpF64ConvertI32_u: return "f64_convert_i32_u";
        case WasmOpF64ConvertI64_s: return "f64_convert_i64_s";
        case WasmOpF64ConvertI64_u: return "f64_convert_i64_u";
        case WasmOpF64PromoteF32: return "f64_promote_f32";
        case WasmOpI32ReinterpretF32: return "i32_reinterpret_f32";
        case WasmOpI64ReinterpretF64: return "i64_reinterpret_f64";
        case WasmOpF32ReinterpretI32: return "f32_reinterpret_i32";
        case WasmOpF64ReinterpretI64: return "f64_reinterpret_i64";
        case WasmOpI32Extend8_s: return "i32_extend8_s";
        case WasmOpI32Extend16_s: return "i32_extend16_s";
        case WasmOpI64Extend8_s: return "i64_extend8_s";
        case WasmOpI64Extend16_s: return "i64_extend16_s";
        case WasmOpI64Extend32_s: return "i64_extend32_s";
        case WasmOpI32TruncSatF32_s: return "i32_trunc_sat_f32_s";
        case WasmOpI32TruncSatF32_u: return "i32_trunc_sat_f32_u";
        case WasmOpI32TruncSatF64_s: return "i32_trunc_sat_f64_s";
        case WasmOpI32TruncSatF64_u: return "i32_trunc_sat_f64_u";
        case WasmOpI64TruncSatF32_s: return "i64_trunc_sat_f32_s";
        case WasmOpI64TruncSatF32_u: return "i64_trunc_sat_f32_u";
        case WasmOpI64TruncSatF64_s: return "i64_trunc_sat_f64_s";
        case WasmOpI64TruncSatF64_u: return "i64_trunc_sat_f64_u";
        default:
            return "unknown";
    }
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

void wmod_dump_global_type(WasmGlobalType *globaltype) {
    wmod_dump_global_mutability(globaltype->mut);
    printf(" ");
    wmod_dump_val_type(&globaltype->valtype);
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
            wmod_dump_global_type(&desc->value.global);
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

void wmod_dump_global(WasmGlobal *global) {
    wmod_dump_global_type(&global->globaltype);
    printf(" ");
    printf("expr(%zu)", global->init.len);
}

void wmod_dump_globals(WasmGlobals *globals) {
    WasmGlobal *data = globals->ptr;
    for (size_t i = 0; i < globals->len; i++) {
        printf("<g%zu> ", i);
        wmod_dump_global(&data[i]);
        printf("\n");
    }
}

void wmod_dump_datamode(WasmDataMode *datamode) {
    switch (datamode->kind) {
        case WasmDataModeActive:
            printf(
                "active <m%d> expr(%zu)",
                datamode->value.active.memidx,
                datamode->value.active.offset_expr.len
            );
            break;
        case WasmDataModePassive:
            printf("passive");
            break;
        default:
            printf("unknown");
    }
}

void wmod_dump_data(WasmData *data) {
    printf("bytes(%d) ", data->len);
    wmod_dump_datamode(&data->datamode);
}

void wmod_dump_datas(WasmDatas *datas) {
    WasmData *data = datas->ptr;
    for (size_t i = 0; i < datas->len; i++) {
        printf("<d%zu> ", i);
        wmod_dump_data(&data[i]);
        printf("\n");
    }
}

void wmod_dump_elemmode(WasmElemMode *elemmode) {
    switch (elemmode->kind) {
        case WasmElemModeActive:
            printf(
                "active <t%d> offset(%zu)",
                elemmode->value.active.tableidx,
                elemmode->value.active.offset_expr.len
            );
            break;
        case WasmElemModePassive:
            printf("passive");
            break;
        case WasmElemModeDeclarative:
            printf("declarative");
            break;
        default:
            break;
    }
}

void wmod_dump_elem(WasmElem *elem) {
    printf("%s ", wmod_str_ref_type(elem->reftype));
    printf("init(%zu) ", elem->init.len);
    wmod_dump_elemmode(&elem->elemmode);
}

void wmod_dump_elems(WasmElems *elems) {
    WasmElem *elem = elems->ptr;
    for (size_t i = 0; i < elems->len; i++) {
        printf("<e%zu> ", i);
        wmod_dump_elem(&elem[i]);
        printf("\n");
    }
}

void wmod_dump(WasmModule *wmod) {
    printf("version: %d\n", wmod->meta.version);
    printf("datacount: %d\n", wmod->meta.datacount);
    printf("-------types: %zu-------\n", wmod->types.len);
    wmod_dump_types(&wmod->types);
    printf("-------funcs: %zu-------\n", wmod->funcs.len);
    wmod_dump_funcs(&wmod->funcs);
    printf("-------globals: %zu-------\n", wmod->globals.len);
    wmod_dump_globals(&wmod->globals);
    printf("-------tables: %zu-------\n", wmod->tables.len);
    wmod_dump_tables(&wmod->tables);
    printf("-------mems: %zu-------\n", wmod->mems.len);
    wmod_dump_mems(&wmod->mems);
    printf("-------imports: %zu-------\n", wmod->imports.len);
    wmod_dump_imports(&wmod->imports);
    printf("-------exports: %zu-------\n", wmod->exports.len);
    wmod_dump_exports(&wmod->exports);
    printf("-------datas: %zu-------\n", wmod->datas.len);
    wmod_dump_datas(&wmod->datas);
    printf("-------elems: %zu-------\n", wmod->elems.len);
    wmod_dump_elems(&wmod->elems);
    printf("-------start: %d-------\n", wmod->start.present);
    wmod_dump_start(&wmod->start);
}

void wmod_name_init(WasmName *name) {
    name->len = 0;
    name->bytes = NULL;
}

bool wmod_name_eq(WasmName *name, char *str) {
    size_t slen = strlen(str);
    return slen == name->len && memcmp(name->bytes, str, slen) == 0;
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

void wmod_global_init(WasmGlobal *global) {
    vec_init(&global->init);
}

void wmod_import_init(WasmImport *import) {
    wmod_name_init(&import->module_name);
    wmod_name_init(&import->item_name);
}

void wmod_export_init(WasmExport *exp) {
    wmod_name_init(&exp->name);
}

void wmod_data_init(WasmData *data) {
    vec_init(&data->datamode.value.active.offset_expr);
}

void wmod_elem_init(WasmElem *elem) {
    vec_init(&elem->init);
    vec_init(&elem->elemmode.value.active.offset_expr);
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

wasm_global_idx_t wmod_push_back_global(WasmModule *wmod, WasmGlobal *global) {
    return vec_push_back(&wmod->globals, sizeof(WasmGlobal), global);
}

wasm_table_idx_t wmod_push_back_table(WasmModule *wmod, WasmTable *table) {
    return vec_push_back(&wmod->tables, sizeof(WasmTable), table);
}

wasm_mem_idx_t wmod_push_back_mem(WasmModule *wmod, WasmMemType *mem) {
    return vec_push_back(&wmod->mems, sizeof(WasmMemType), mem);
}

wasm_data_idx_t wmod_push_back_data(WasmModule *wmod, WasmData *data) {
    return vec_push_back(&wmod->datas, sizeof(WasmData), data);
}

wasm_elem_idx_t wmod_push_back_elem(WasmModule *wmod, WasmElem *elem) {
    return vec_push_back(&wmod->elems, sizeof(WasmElem), elem);
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

void wmod_expr_push_back_instruction(WasmExpr *expr, WasmInstruction *ins) {
    vec_push_back(expr, sizeof(WasmInstruction), ins);
}

void wmod_elem_push_back_expr(WasmElem *elem, WasmExpr *expr) {
    vec_push_back(&elem->init, sizeof(WasmExpr), expr);
}

void wmod_instr_init(WasmInstruction *instr, WasmOpcode opcode) {
    instr->opcode = opcode;
    switch (opcode) {
        case WasmOpBlock:
        case WasmOpLoop:
            vec_init(&instr->params.block.expr);
            break;
        case WasmOpIf:
            vec_init(&instr->params._if.then_body);
            vec_init(&instr->params._if.else_body);
            break;
        case WasmOpBreakTable:
            vec_init(&instr->params.break_table.labels);
            break;
        case WasmOpSelect:
            vec_init(&instr->params.select.valuetypes);
            break;
        default:
            break;
    }
}

WasmValidateResult wmod_validate(WasmModule *wmod) {
    return WasmModuleOk;
}
