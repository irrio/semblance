
#include "wbin.h"
#include "wmod.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

WasmDecodeResult wbin_decode_expr(void *data, WasmExpr *expr);
WasmDecodeResult wbin_decode_instr(void *data, WasmInstruction *ins);

WasmDecodeResult wbin_err(WasmDecodeErrorCode error_code, int cause) {
    WasmDecodeResult out;
    out.state = WasmDecodeErr;
    out.value.error.code = error_code;
    out.value.error.cause =  cause;
    return out;
}

WasmDecodeResult wbin_err_io(int cause) {
    return wbin_err(WasmDecodeErrIo, cause);
}

WasmDecodeResult wbin_ok(void *next_data) {
    WasmDecodeResult out;
    out.state = WasmDecodeOk;
    out.value.next_data = next_data;
    return out;
}

void *wbin_take_byte(void *data, uint8_t *out) {
    uint8_t *bytes = data;
    *out = bytes[0];
    return bytes + 1;
}

void *wbin_decode_leb128_64(void *data, uint64_t *out) {
    uint8_t *bytes = data;
    uint32_t shift = 0;
    size_t byte_idx = 0;
    *out = 0;
    while (true) {
        uint8_t byte = bytes[byte_idx];
        *out |= (byte & ~(1 << 7)) << shift;
        if ((byte & (1 << 7)) == 0) {
            break;
        };
        shift += 7;
        byte_idx++;
    }
    return data + byte_idx + 1;
}

void *wbin_decode_leb128_signed_64(u_leb128_prefixed data, int64_t *out) {
    int64_t result = 0;
    uint32_t shift = 0;

    size_t idx = 0;
    uint8_t byte = data[idx];
    do {
        byte = data[idx];
        result |= (byte & ~(1 << 7)) << shift;
        shift += 7;
        idx++;
    } while ((byte & (1 << 7)) != 0);

    if ((shift < 64) && ((byte & 0x40) != 0)) {
        result |= (~0 << shift);
    }

    *out = result;

    return data + idx;
}

void *wbin_decode_leb128(u_leb128_prefixed data, uint32_t *out) {
    uint64_t full;
    data = wbin_decode_leb128_64(data, &full);
    *out = full;
    return data;
}

void *wbin_decode_leb128_signed(u_leb128_prefixed data, int32_t *out) {
    int64_t full;
    data = wbin_decode_leb128_signed_64(data, &full);
    *out = full;
    return data;
}

void *wbin_decode_leb128_signed_tag(void *data, uint32_t *out) {
    int64_t full;
    data = wbin_decode_leb128_signed_64(data, &full);
    *out = full;
    return data;
}

WasmDecodeResult wbin_decode_reftype(void *data, WasmRefType *out) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    switch (tag) {
        case 0x70:
            *out = WasmRefFunc;
            break;
        case 0x6F:
            *out = WasmRefExtern;
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidType, 0);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_val_type(void *data, WasmValueType *out) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    switch (tag) {
        case 0x7F:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumI32;
            break;
        case 0x7E:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumI64;
            break;
        case 0x7D:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumF32;
            break;
        case 0x7C:
            out->kind = WasmValueTypeNum;
            out->value.num = WasmNumF64;
            break;
        case 0x7B:
            out->kind = WasmValueTypeVec;
            out->value.vec = WasmVecV128;
            break;
        case 0x70:
            out->kind = WasmValueTypeRef;
            out->value.ref = WasmRefFunc;
            break;
        case 0x6F:
            out->kind = WasmValueTypeRef;
            out->value.ref = WasmRefExtern;
            break;
        default:
            return wbin_err(WasmDecodeErrUnknownValueType, 0);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_result_type(void *data, WasmResultType *out) {
    uint32_t len = 0;
    data = wbin_decode_leb128(data, &len);
    while (len > 0) {
        WasmValueType valtype;
        WasmDecodeResult valtype_result = wbin_decode_val_type(data, &valtype);
        if (!wbin_is_ok(valtype_result)) return valtype_result;
        wmod_result_type_push_back(out, &valtype);
        data = valtype_result.value.next_data;
        len--;
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_type(void *data, WasmFuncType *out) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    if (tag != 0x60) return wbin_err(WasmDecodeErrInvalidType, 0);
    WasmDecodeResult input_result = wbin_decode_result_type(data, &out->input_type);
    if (!wbin_is_ok(input_result)) return input_result;
    return wbin_decode_result_type(input_result.value.next_data, &out->output_type);
}

WasmDecodeResult wbin_decode_types(void *data, WasmModule *wmod) {
    uint32_t len = 0;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmFuncType decoded_type;
        wmod_func_type_init(&decoded_type);
        WasmDecodeResult result = wbin_decode_type(data, &decoded_type);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_type(wmod, &decoded_type);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_funcs(void *data, WasmModule *wmod) {
    uint32_t len = 0;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmFunc decoded_func;
        wmod_func_init(&decoded_func);
        data = wbin_decode_leb128(data, &decoded_func.type_idx);
        wmod_push_back_func(wmod, &decoded_func);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_limits(void *data, WasmLimits *limits) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    switch (tag) {
        case 0x00:
            limits->bounded = false;
            break;
        case 0x01:
            limits->bounded = true;
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidLimit, 0);
    }
    data = wbin_decode_leb128(data, &limits->min);
    if (limits->bounded) {
        data = wbin_decode_leb128(data, &limits->max);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_table(void *data, WasmTable *table) {
    WasmDecodeResult ref_result = wbin_decode_reftype(data, &table->reftype);
    if (!wbin_is_ok(ref_result)) return ref_result;
    data = ref_result.value.next_data;
    return wbin_decode_limits(data, &table->limits);
}

WasmDecodeResult wbin_decode_tables(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmTable decoded_table = { 0 };
        WasmDecodeResult result = wbin_decode_table(data, &decoded_table);
        if (!wbin_is_ok(result)) return result;
        wmod_push_back_table(wmod, &decoded_table);
        data = result.value.next_data;
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_mem(void* data, WasmMemType *mem) {
    return wbin_decode_limits(data, &mem->limits);
}

WasmDecodeResult wbin_decode_mems(void* data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmMemType mem = { 0 };
        WasmDecodeResult result = wbin_decode_mem(data, &mem);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_mem(wmod, &mem);
        len--;
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_name(void* data,  WasmName *name) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);
    name->len = len;
    name->bytes = data;
    return wbin_ok(data + len);
}

WasmDecodeResult wbin_decode_global_mutability(void *data, WasmGlobalMutability *mut) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    switch (tag) {
        case 0x00:
            *mut = WasmGlobalConst;
            break;
        case 0x01:
            *mut = WasmGlobalVar;
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidGlobalMutability, 0);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_global_type(void *data, WasmGlobalType *global) {
    WasmDecodeResult val_result = wbin_decode_val_type(data, &global->valtype);
    if (!wbin_is_ok(val_result)) return val_result;
    data = val_result.value.next_data;
    return wbin_decode_global_mutability(data, &global->mut);
}

WasmDecodeResult wbin_decode_import_desc(void *data, WasmImportDesc *desc) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    switch (tag) {
        case 0x00:
            desc->kind = WasmImportFunc;
            return wbin_ok(wbin_decode_leb128(data, &desc->value.func));
        case 0x01:
            desc->kind = WasmImportTable;
            return wbin_decode_table(data, &desc->value.table);
        case 0x02:
            desc->kind = WasmImportMem;
            return wbin_decode_mem(data, &desc->value.mem);
        case 0x03:
            desc->kind = WasmImportGlobal;
            return wbin_decode_global_type(data, &desc->value.global);
        default:
            return wbin_err(WasmDecodeErrInvalidImport, 0);
    }
}

WasmDecodeResult wbin_decode_import(void *data, WasmImport *import) {
    WasmDecodeResult modname_result = wbin_decode_name(data, &import->module_name);
    if (!wbin_is_ok(modname_result)) return modname_result;
    data = modname_result.value.next_data;
    WasmDecodeResult name_result = wbin_decode_name(data, &import->item_name);
    if (!wbin_is_ok(name_result)) return name_result;
    data = name_result.value.next_data;
    return wbin_decode_import_desc(data, &import->desc);
}

WasmDecodeResult wbin_decode_imports(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmImport import;
        wmod_import_init(&import);
        WasmDecodeResult result = wbin_decode_import(data, &import);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_import(wmod, &import);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_export_desc(void *data, WasmExportDesc *desc) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);
    switch (tag) {
        case 0x00:
            desc->kind = WasmExportFunc;
            return wbin_ok(wbin_decode_leb128(data, &desc->value.func));
        case 0x01:
            desc->kind = WasmExportTable;
            return wbin_ok(wbin_decode_leb128(data, &desc->value.table));
        case 0x02:
            desc->kind = WasmExportMem;
            return wbin_ok(wbin_decode_leb128(data, &desc->value.mem));
        case 0x03:
            desc->kind = WasmExportGlobal;
            return wbin_ok(wbin_decode_leb128(data, &desc->value.global));
        default:
            return wbin_err(WasmDecodeErrInvalidExport, 0);
    }
}

WasmDecodeResult wbin_decode_export(void *data, WasmExport *exp) {
    WasmDecodeResult name_result = wbin_decode_name(data, &exp->name);
    if (!wbin_is_ok(name_result)) return name_result;
    data = name_result.value.next_data;
    return wbin_decode_export_desc(data, &exp->desc);
}

WasmDecodeResult wbin_decode_exports(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        WasmExport exp;
        wmod_export_init(&exp);
        WasmDecodeResult result = wbin_decode_export(data, &exp);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_export(wmod, &exp);
        len--;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_start(void *data, WasmModule *wmod) {
    wmod->start.present = true;
    data = wbin_decode_leb128(data, &wmod->start.func_idx);
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_locals(void *data, WasmFunc *func) {
    uint32_t len;
    data =  wbin_decode_leb128(data, &len);
    for (size_t i = 0; i < len; i++) {
        uint32_t n;
        WasmValueType valtype;
        data = wbin_decode_leb128(data, &n);
        WasmDecodeResult val_result = wbin_decode_val_type(data, &valtype);
        if (!wbin_is_ok(val_result)) return val_result;
        data = val_result.value.next_data;
        wmod_func_push_back_locals(func, n, &valtype);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_blocktype(void *data, WasmBlockType *blocktype) {
    if (*(uint8_t*)data == 0x40) {
        blocktype->kind = WasmBlockTypeEmpty;
        return wbin_ok((uint8_t*)data + 1);
    }
    WasmDecodeResult val_result = wbin_decode_val_type(data, &blocktype->value.valtype);
    if (wbin_is_ok(val_result)) {
        blocktype->kind = WasmBlockTypeVal;
        return val_result;
    }
    if (!wbin_is_err(val_result, WasmDecodeErrInvalidType)) return val_result;

    blocktype->kind = WasmBlockTypeIdx;
    data = wbin_decode_leb128_signed_tag(data, &blocktype->value.typeidx);

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_block(void *data, WasmBlockParams *block) {
    WasmDecodeResult bt_result = wbin_decode_blocktype(data, &block->blocktype);
    if (!wbin_is_ok(bt_result)) return bt_result;
    data = bt_result.value.next_data;
    return wbin_decode_expr(data, &block->expr);
}

WasmDecodeResult wbin_decode_if(void *data, WasmIfParams *_if) {
    WasmDecodeResult bt_result = wbin_decode_blocktype(data, &_if->blocktype);
    if (!wbin_is_ok(bt_result)) return bt_result;
    data = bt_result.value.next_data;

    WasmInstruction instr;
    WasmExpr *expr = &_if->then_body;
    while (true) {
        WasmDecodeResult result = wbin_decode_instr(data, &instr);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        if (instr.opcode == WasmOpElse) {
            expr = &_if->else_body;
        } if (instr.opcode == WasmOpExprEnd) {
            break;
        } else {
            wmod_expr_push_back_instruction(expr, &instr);
        }
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_break(void *data, WasmBreakParams *br) {
    data = wbin_decode_leb128(data, &br->label);
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_break_table(void *data, WasmBreakTableParams *bt) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len > 0) {
        wasm_type_idx_t typeidx;
        data = wbin_decode_leb128(data, &typeidx);
        vec_push_back(&bt->labels, sizeof(wasm_type_idx_t), &typeidx);
        len--;
    }

    data = wbin_decode_leb128(data, &bt->default_label);

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_call(void *data, WasmCallParams *call) {
    data = wbin_decode_leb128(data, &call->funcidx);
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_call_indirect(void *data, WasmCallIndirectParams *call) {
    data = wbin_decode_leb128(data, &call->typeidx);
    data = wbin_decode_leb128(data, &call->tableidx);
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_val_types(void *data, VEC(WasmValueType) *valuetypes) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);
    while (len-- > 0) {
        WasmValueType valtype;
        WasmDecodeResult result = wbin_decode_val_type(data, &valtype);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        vec_push_back(valuetypes, sizeof(WasmValueType), &valtype);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_zero(void *data) {
    uint8_t byte;
    data = wbin_take_byte(data, &byte);
    if (byte != 0) {
        return wbin_err(WasmDecodeErrExpectedZero, 0);
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_zeroes(void *data) {
    WasmDecodeResult res = wbin_decode_zero(data);
    if (!wbin_is_ok(res)) return res;
    data = res.value.next_data;
    return wbin_decode_zero(data);
}

WasmDecodeResult wbin_decode_extended_instr(void *data, WasmInstruction *ins) {
    uint32_t tag;
    data = wbin_decode_leb128(data, &tag);

    switch (tag) {
        case 0:
            wmod_instr_init(ins, WasmOpI32TruncSatF32_s);
            break;
        case 1:
            wmod_instr_init(ins, WasmOpI32TruncSatF32_u);
            break;
        case 2:
            wmod_instr_init(ins, WasmOpI32TruncSatF64_s);
            break;
        case 3:
            wmod_instr_init(ins, WasmOpI32TruncSatF64_u);
            break;
        case 4:
            wmod_instr_init(ins, WasmOpI64TruncSatF32_s);
            break;
        case 5:
            wmod_instr_init(ins, WasmOpI64TruncSatF32_u);
            break;
        case 6:
            wmod_instr_init(ins, WasmOpI64TruncSatF64_s);
            break;
        case 7:
            wmod_instr_init(ins, WasmOpI64TruncSatF64_u);
            break;
        case 8:
            wmod_instr_init(ins, WasmOpMemoryInit);
            data = wbin_decode_leb128(data, &ins->params.mem_init.dataidx);
            return wbin_decode_zero(data);
        case 9:
            wmod_instr_init(ins, WasmOpDataDrop);
            data = wbin_decode_leb128(data, &ins->params.mem_init.dataidx);
            break;
        case 10:
            wmod_instr_init(ins, WasmOpMemoryCopy);
            return wbin_decode_zeroes(data);
        case 11:
            wmod_instr_init(ins, WasmOpMemoryFill);
            return wbin_decode_zero(data);
        case 12:
            wmod_instr_init(ins, WasmOpTableInit);
            data = wbin_decode_leb128(data, &ins->params.table_init.elemidx);
            data = wbin_decode_leb128(data, &ins->params.table_init.tableidx);
            break;
        case 13:
            wmod_instr_init(ins, WasmOpElemDrop);
            data = wbin_decode_leb128(data, &ins->params.elem_drop.elemidx);
            break;
        case 14:
            wmod_instr_init(ins, WasmOpTableCopy);
            data = wbin_decode_leb128(data, &ins->params.table_copy.src);
            data = wbin_decode_leb128(data, &ins->params.table_copy.dst);
            break;
        case 15:
            wmod_instr_init(ins, WasmOpTableGrow);
            data = wbin_decode_leb128(data, &ins->params.table.tableidx);
            break;
        case 16:
            wmod_instr_init(ins, WasmOpTableSize);
            data = wbin_decode_leb128(data, &ins->params.table.tableidx);
            break;
        case 17:
            wmod_instr_init(ins, WasmOpTableFill);
            data = wbin_decode_leb128(data, &ins->params.table.tableidx);
            break;
        default:
            return wbin_err(WasmDecodeErrInvalidTableInstr, 0);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_memarg(void *data, WasmMemArg *memarg) {
    data = wbin_decode_leb128(data, &memarg->align);
    data = wbin_decode_leb128(data, &memarg->offset);
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_i32(void *data, int32_t *out) {
    return wbin_ok(wbin_decode_leb128_signed(data, out));
}

WasmDecodeResult wbin_decode_i64(void *data, int64_t *out) {
    return wbin_ok(wbin_decode_leb128_signed_64(data, out));
}

WasmDecodeResult wbin_decode_f32(void *data, float *out) {
    *out = *(float*)data;
    return wbin_ok(data + sizeof(float));
}

WasmDecodeResult wbin_decode_f64(void *data, double *out) {
    *out = *(double*)data;
    return wbin_ok(data + sizeof(double));
}

WasmDecodeResult wbin_decode_instr(void *data, WasmInstruction *ins) {
    uint8_t tag;
    data = wbin_take_byte(data, &tag);

    switch (tag) {
        case 0x00:
            wmod_instr_init(ins, WasmOpUnreachable);
            break;
        case 0x01:
            wmod_instr_init(ins, WasmOpNop);
            break;
        case 0x02:
            wmod_instr_init(ins, WasmOpBlock);
            return wbin_decode_block(data, &ins->params.block);
        case 0x03:
            wmod_instr_init(ins, WasmOpLoop);
            return wbin_decode_block(data, &ins->params.block);
        case 0x04:
            wmod_instr_init(ins, WasmOpIf);
            return wbin_decode_if(data, &ins->params._if);
        case 0x05:
            wmod_instr_init(ins, WasmOpElse);
            break;
        case 0x0C:
            wmod_instr_init(ins, WasmOpBreak);
            return wbin_decode_break(data, &ins->params._break);
        case 0x0D:
            wmod_instr_init(ins, WasmOpBreakIf);
            return wbin_decode_break(data, &ins->params._break);
        case 0x0E:
            wmod_instr_init(ins, WasmOpBreakTable);
            return wbin_decode_break_table(data, &ins->params.break_table);
        case 0x0F:
            wmod_instr_init(ins, WasmOpReturn);
            break;
        case 0x10:
            wmod_instr_init(ins, WasmOpCall);
            return wbin_decode_call(data, &ins->params.call);
        case 0x11:
            wmod_instr_init(ins, WasmOpCallIndirect);
            return wbin_decode_call_indirect(data, &ins->params.call_indirect);
        case 0xD0:
            wmod_instr_init(ins, WasmOpRefNull);
            return wbin_decode_reftype(data, &ins->params.ref_null.reftype);
        case 0xD1:
            wmod_instr_init(ins, WasmOpRefIsNull);
            break;
        case 0xD2:
            wmod_instr_init(ins, WasmOpRefFunc);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.ref_func.funcidx));
        case 0x1A:
            wmod_instr_init(ins, WasmOpDrop);
            break;
        case 0x1B:
            wmod_instr_init(ins, WasmOpSelect);
            break;
        case 0x1C:
            wmod_instr_init(ins, WasmOpSelect);
            return wbin_decode_val_types(data, &ins->params.select.valuetypes);
        case 0x20:
            wmod_instr_init(ins, WasmOpLocalGet);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.var.idx.local));
        case 0x21:
            wmod_instr_init(ins, WasmOpLocalSet);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.var.idx.local));
        case 0x22:
            wmod_instr_init(ins, WasmOpLocalTee);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.var.idx.local));
        case 0x23:
            wmod_instr_init(ins, WasmOpGlobalGet);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.var.idx.global));
        case 0x24:
            wmod_instr_init(ins, WasmOpGlobalSet);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.var.idx.global));
        case 0x25:
            wmod_instr_init(ins, WasmOpTableGet);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.table.tableidx));
        case 0x26:
            wmod_instr_init(ins, WasmOpTableSet);
            return wbin_ok(wbin_decode_leb128(data, &ins->params.table.tableidx));
        case 0x28:
            wmod_instr_init(ins, WasmOpI32Load);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x29:
            wmod_instr_init(ins, WasmOpI64Load);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x2A:
            wmod_instr_init(ins, WasmOpF32Load);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x2B:
            wmod_instr_init(ins, WasmOpF64Load);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x2C:
            wmod_instr_init(ins, WasmOpI32Load8_s);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x2D:
            wmod_instr_init(ins, WasmOpI32Load8_u);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x2E:
            wmod_instr_init(ins, WasmOpI32Load16_s);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x2F:
            wmod_instr_init(ins, WasmOpI32Load16_u);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x30:
            wmod_instr_init(ins, WasmOpI64Load8_s);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x31:
            wmod_instr_init(ins, WasmOpI64Load8_u);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x32:
            wmod_instr_init(ins, WasmOpI64Load16_s);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x33:
            wmod_instr_init(ins, WasmOpI64Load16_u);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x34:
            wmod_instr_init(ins, WasmOpI64Load32_s);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x35:
            wmod_instr_init(ins, WasmOpI64Load32_u);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x36:
            wmod_instr_init(ins, WasmOpI32Store);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x37:
            wmod_instr_init(ins, WasmOpI64Store);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x38:
            wmod_instr_init(ins, WasmOpF32Store);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x39:
            wmod_instr_init(ins, WasmOpF64Store);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x3A:
            wmod_instr_init(ins, WasmOpI32Store8);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x3B:
            wmod_instr_init(ins, WasmOpI32Store16);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x3C:
            wmod_instr_init(ins, WasmOpI64Store8);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x3D:
            wmod_instr_init(ins, WasmOpI64Store16);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x3E:
            wmod_instr_init(ins, WasmOpI64Store32);
            return wbin_decode_memarg(data, &ins->params.memarg);
        case 0x3F:
            wmod_instr_init(ins, WasmOpMemorySize);
            return wbin_decode_zero(data);
        case 0x40:
            wmod_instr_init(ins, WasmOpMemoryGrow);
            return wbin_decode_zero(data);
        case 0x41:
            wmod_instr_init(ins, WasmOpI32Const);
            return wbin_decode_i32(data, &ins->params._const.value.i32);
        case 0x42:
            wmod_instr_init(ins, WasmOpI64Const);
            return wbin_decode_i64(data, &ins->params._const.value.i64);
        case 0x43:
            wmod_instr_init(ins, WasmOpF32Const);
            return wbin_decode_f32(data, &ins->params._const.value.f32);
        case 0x44:
            wmod_instr_init(ins, WasmOpF64Const);
            return wbin_decode_f64(data, &ins->params._const.value.f64);
        case 0x45:
            wmod_instr_init(ins, WasmOpI32EqZ);
            break;
        case 0x46:
            wmod_instr_init(ins, WasmOpI32Eq);
            break;
        case 0x47:
            wmod_instr_init(ins, WasmOpI32Neq);
            break;
        case 0x48:
            wmod_instr_init(ins, WasmOpI32Lt_s);
            break;
        case 0x49:
            wmod_instr_init(ins, WasmOpI32Lt_u);
            break;
        case 0x4A:
            wmod_instr_init(ins, WasmOpI32Gt_s);
            break;
        case 0x4B:
            wmod_instr_init(ins, WasmOpI32Gt_u);
            break;
        case 0x4C:
            wmod_instr_init(ins, WasmOpI32Le_s);
            break;
        case 0x4D:
            wmod_instr_init(ins, WasmOpI32Le_u);
            break;
        case 0x4E:
            wmod_instr_init(ins, WasmOpI32Ge_s);
            break;
        case 0x4F:
            wmod_instr_init(ins, WasmOpI32Ge_u);
            break;
        case 0x50:
            wmod_instr_init(ins, WasmOpI64EqZ);
            break;
        case 0x51:
            wmod_instr_init(ins, WasmOpI64Eq);
            break;
        case 0x52:
            wmod_instr_init(ins, WasmOpI64Neq);
            break;
        case 0x53:
            wmod_instr_init(ins, WasmOpI64Lt_s);
            break;
        case 0x54:
            wmod_instr_init(ins, WasmOpI64Lt_u);
            break;
        case 0x55:
            wmod_instr_init(ins, WasmOpI64Gt_s);
            break;
        case 0x56:
            wmod_instr_init(ins, WasmOpI64Gt_u);
            break;
        case 0x57:
            wmod_instr_init(ins, WasmOpI64Le_s);
            break;
        case 0x58:
            wmod_instr_init(ins, WasmOpI64Le_u);
            break;
        case 0x59:
            wmod_instr_init(ins, WasmOpI64Ge_s);
            break;
        case 0x5A:
            wmod_instr_init(ins, WasmOpI64Ge_u);
            break;
        case 0x5B:
            wmod_instr_init(ins, WasmOpF32Eq);
            break;
        case 0x5C:
            wmod_instr_init(ins, WasmOpF32Neq);
            break;
        case 0x5D:
            wmod_instr_init(ins, WasmOpF32Lt);
            break;
        case 0x5E:
            wmod_instr_init(ins, WasmOpF32Gt);
            break;
        case 0x5F:
            wmod_instr_init(ins, WasmOpF32Le);
            break;
        case 0x60:
            wmod_instr_init(ins, WasmOpF32Ge);
            break;
        case 0x61:
            wmod_instr_init(ins, WasmOpF64Eq);
            break;
        case 0x62:
            wmod_instr_init(ins, WasmOpF64Neq);
            break;
        case 0x63:
            wmod_instr_init(ins, WasmOpF64Lt);
            break;
        case 0x64:
            wmod_instr_init(ins, WasmOpF64Gt);
            break;
        case 0x65:
            wmod_instr_init(ins, WasmOpF64Le);
            break;
        case 0x66:
            wmod_instr_init(ins, WasmOpF64Ge);
            break;
        case 0x67:
            wmod_instr_init(ins, WasmOpI32Clz);
            break;
        case 0x68:
            wmod_instr_init(ins, WasmOpI32Ctz);
            break;
        case 0x69:
            wmod_instr_init(ins, WasmOpI32Popcnt);
            break;
        case 0x6A:
            wmod_instr_init(ins, WasmOpI32Add);
            break;
        case 0x6B:
            wmod_instr_init(ins, WasmOpI32Sub);
            break;
        case 0x6C:
            wmod_instr_init(ins, WasmOpI32Mul);
            break;
        case 0x6D:
            wmod_instr_init(ins, WasmOpI32Div_s);
            break;
        case 0x6E:
            wmod_instr_init(ins, WasmOpI32Div_u);
            break;
        case 0x6F:
            wmod_instr_init(ins, WasmOpI32Rem_s);
            break;
        case 0x70:
            wmod_instr_init(ins, WasmOpI32Rem_u);
            break;
        case 0x71:
            wmod_instr_init(ins, WasmOpI32And);
            break;
        case 0x72:
            wmod_instr_init(ins, WasmOpI32Or);
            break;
        case 0x73:
            wmod_instr_init(ins, WasmOpI32Xor);
            break;
        case 0x74:
            wmod_instr_init(ins, WasmOpI32Shl);
            break;
        case 0x75:
            wmod_instr_init(ins, WasmOpI32Shr_s);
            break;
        case 0x76:
            wmod_instr_init(ins, WasmOpI32Shr_u);
            break;
        case 0x77:
            wmod_instr_init(ins, WasmOpI32Rotl);
            break;
        case 0x78:
            wmod_instr_init(ins, WasmOpI32Rotr);
            break;
        case 0x79:
            wmod_instr_init(ins, WasmOpI64Clz);
            break;
        case 0x7A:
            wmod_instr_init(ins, WasmOpI64Ctz);
            break;
        case 0x7B:
            wmod_instr_init(ins, WasmOpI64Popcnt);
            break;
        case 0x7C:
            wmod_instr_init(ins, WasmOpI64Add);
            break;
        case 0x7D:
            wmod_instr_init(ins, WasmOpI64Sub);
            break;
        case 0x7E:
            wmod_instr_init(ins, WasmOpI64Mul);
            break;
        case 0x7F:
            wmod_instr_init(ins, WasmOpI64Div_s);
            break;
        case 0x80:
            wmod_instr_init(ins, WasmOpI64Div_u);
            break;
        case 0x81:
            wmod_instr_init(ins, WasmOpI64Rem_s);
            break;
        case 0x82:
            wmod_instr_init(ins, WasmOpI64Rem_u);
            break;
        case 0x83:
            wmod_instr_init(ins, WasmOpI64And);
            break;
        case 0x84:
            wmod_instr_init(ins, WasmOpI64Or);
            break;
        case 0x85:
            wmod_instr_init(ins, WasmOpI64Xor);
            break;
        case 0x86:
            wmod_instr_init(ins, WasmOpI64Shl);
            break;
        case 0x87:
            wmod_instr_init(ins, WasmOpI64Shr_s);
            break;
        case 0x88:
            wmod_instr_init(ins, WasmOpI64Shr_u);
            break;
        case 0x89:
            wmod_instr_init(ins, WasmOpI64Rotl);
            break;
        case 0x8A:
            wmod_instr_init(ins, WasmOpI64Rotr);
            break;
        case 0x8B:
            wmod_instr_init(ins, WasmOpF32Abs);
            break;
        case 0x8C:
            wmod_instr_init(ins, WasmOpF32Neg);
            break;
        case 0x8D:
            wmod_instr_init(ins, WasmOpF32Ceil);
            break;
        case 0x8E:
            wmod_instr_init(ins, WasmOpF32Floor);
            break;
        case 0x8F:
            wmod_instr_init(ins, WasmOpF32Trunc);
            break;
        case 0x90:
            wmod_instr_init(ins, WasmOpF32Nearest);
            break;
        case 0x91:
            wmod_instr_init(ins, WasmOpF32Sqrt);
            break;
        case 0x92:
            wmod_instr_init(ins, WasmOpF32Add);
            break;
        case 0x93:
            wmod_instr_init(ins, WasmOpF32Sub);
            break;
        case 0x94:
            wmod_instr_init(ins, WasmOpF32Mul);
            break;
        case 0x95:
            wmod_instr_init(ins, WasmOpF32Div);
            break;
        case 0x96:
            wmod_instr_init(ins, WasmOpF32Min);
            break;
        case 0x97:
            wmod_instr_init(ins, WasmOpF32Max);
            break;
        case 0x98:
            wmod_instr_init(ins, WasmOpF32CopySign);
            break;
        case 0x99:
            wmod_instr_init(ins, WasmOpF64Abs);
            break;
        case 0x9A:
            wmod_instr_init(ins, WasmOpF64Neg);
            break;
        case 0x9B:
            wmod_instr_init(ins, WasmOpF64Ceil);
            break;
        case 0x9C:
            wmod_instr_init(ins, WasmOpF64Floor);
            break;
        case 0x9D:
            wmod_instr_init(ins, WasmOpF64Trunc);
            break;
        case 0x9E:
            wmod_instr_init(ins, WasmOpF64Nearest);
            break;
        case 0x9F:
            wmod_instr_init(ins, WasmOpF64Sqrt);
            break;
        case 0xA0:
            wmod_instr_init(ins, WasmOpF64Add);
            break;
        case 0xA1:
            wmod_instr_init(ins, WasmOpF64Sub);
            break;
        case 0xA2:
            wmod_instr_init(ins, WasmOpF64Mul);
            break;
        case 0xA3:
            wmod_instr_init(ins, WasmOpF64Div);
            break;
        case 0xA4:
            wmod_instr_init(ins, WasmOpF64Min);
            break;
        case 0xA5:
            wmod_instr_init(ins, WasmOpF64Max);
            break;
        case 0xA6:
            wmod_instr_init(ins, WasmOpF64CopySign);
            break;
        case 0xA7:
            wmod_instr_init(ins, WasmOpI32WrapI64);
            break;
        case 0xA8:
            wmod_instr_init(ins, WasmOpI32TruncF32_s);
            break;
        case 0xA9:
            wmod_instr_init(ins, WasmOpI32TruncF32_u);
            break;
        case 0xAA:
            wmod_instr_init(ins, WasmOpI32TruncF64_s);
            break;
        case 0xAB:
            wmod_instr_init(ins, WasmOpI32TruncF64_u);
            break;
        case 0xAC:
            wmod_instr_init(ins, WasmOpI64ExtendI32_s);
            break;
        case 0xAD:
            wmod_instr_init(ins, WasmOpI64ExtendI32_u);
            break;
        case 0xAE:
            wmod_instr_init(ins, WasmOpI64TruncF32_s);
            break;
        case 0xAF:
            wmod_instr_init(ins, WasmOpI64TruncF32_u);
            break;
        case 0xB0:
            wmod_instr_init(ins, WasmOpI64TruncF64_s);
            break;
        case 0xB1:
            wmod_instr_init(ins, WasmOpI64TruncF64_u);
            break;
        case 0xB2:
            wmod_instr_init(ins, WasmOpF32ConvertI32_s);
            break;
        case 0xB3:
            wmod_instr_init(ins, WasmOpF32ConvertI32_u);
            break;
        case 0xB4:
            wmod_instr_init(ins, WasmOpF32ConvertI64_s);
            break;
        case 0xB5:
            wmod_instr_init(ins, WasmOpF32ConvertI64_u);
            break;
        case 0xB6:
            wmod_instr_init(ins, WasmOpF32DemoteF64);
            break;
        case 0xB7:
            wmod_instr_init(ins, WasmOpF64ConvertI32_s);
            break;
        case 0xB8:
            wmod_instr_init(ins, WasmOpF64ConvertI32_u);
            break;
        case 0xB9:
            wmod_instr_init(ins, WasmOpF64ConvertI64_s);
            break;
        case 0xBA:
            wmod_instr_init(ins, WasmOpF32ConvertI64_u);
            break;
        case 0xBB:
            wmod_instr_init(ins, WasmOpF64PromoteF32);
            break;
        case 0xBC:
            wmod_instr_init(ins, WasmOpI32ReinterpretF32);
            break;
        case 0xBD:
            wmod_instr_init(ins, WasmOpI64ReinterpretF64);
            break;
        case 0xBE:
            wmod_instr_init(ins, WasmOpF32ReinterpretI32);
            break;
        case 0xBF:
            wmod_instr_init(ins, WasmOpF64ReinterpretI64);
            break;
        case 0xC0:
            wmod_instr_init(ins, WasmOpI32Extend8_s);
            break;
        case 0xC1:
            wmod_instr_init(ins, WasmOpI32Extend16_s);
            break;
        case 0xC2:
            wmod_instr_init(ins, WasmOpI64Extend8_s);
            break;
        case 0xC3:
            wmod_instr_init(ins, WasmOpI64Extend16_s);
            break;
        case 0xC4:
            wmod_instr_init(ins, WasmOpI64Extend32_s);
            break;
        // END
        case 0xFC:
            return wbin_decode_extended_instr(data, ins);
        case 0x0B:
            wmod_instr_init(ins, WasmOpExprEnd);
            break;
        default:
            return wbin_err(WasmDecodeErrUnknownOpcode, tag);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_expr(void *data, WasmExpr *expr) {
    WasmInstruction instr;
    while (true) {
        WasmDecodeResult result = wbin_decode_instr(data, &instr);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_expr_push_back_instruction(expr, &instr);
        if (instr.opcode == WasmOpExprEnd) {
            break;
        }
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_code(void *data, WasmFunc *func) {
    WasmDecodeResult locals_result = wbin_decode_locals(data, func);
    if (!wbin_is_ok(locals_result)) return locals_result;
    data = locals_result.value.next_data;
    WasmDecodeResult expr_result = wbin_decode_expr(data, &func->body);
    if (!wbin_is_ok(expr_result)) return expr_result;
    data = expr_result.value.next_data;
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_codes(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    WasmFunc *func = wmod->funcs.ptr;
    for (size_t i = 0; i < len; i++) {
        uint32_t code_len;
        data = wbin_decode_leb128(data, &code_len);
        WasmDecodeResult result = wbin_decode_code(data, &func[i]);
        if (!wbin_is_ok(result)) return result;
        data += code_len;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_global(void *data, WasmGlobal *global) {
    WasmDecodeResult result = wbin_decode_global_type(data, &global->globaltype);
    if (!wbin_is_ok(result)) return result;
    data = result.value.next_data;
    return wbin_decode_expr(data, &global->init);
}

WasmDecodeResult wbin_decode_globals(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len-- > 0) {
        WasmGlobal global;
        wmod_global_init(&global);
        WasmDecodeResult result = wbin_decode_global(data, &global);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_global(wmod, &global);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_data(void *data, WasmData *wdata) {
    uint32_t tag;
    data = wbin_decode_leb128(data, &tag);

    switch (tag) {
        case 0:
            wdata->datamode.kind = WasmDataModeActive;
            wdata->datamode.value.active.memidx = 0;
            WasmDecodeResult result = wbin_decode_expr(data, &wdata->datamode.value.active.offset_expr);
            if (!wbin_is_ok(result)) return result;
            data = result.value.next_data;
            data = wbin_decode_leb128(data, &wdata->len);
            wdata->bytes = data;
            data += wdata->len;
            break;
        case 1:
            wdata->datamode.kind = WasmDataModePassive;
            data = wbin_decode_leb128(data, &wdata->len);
            wdata->bytes = data;
            data += wdata->len;
            break;
        case 2:
            wdata->datamode.kind = WasmDataModeActive;
            data = wbin_decode_leb128(data, &wdata->datamode.value.active.memidx);
            WasmDecodeResult result2 = wbin_decode_expr(data, &wdata->datamode.value.active.offset_expr);
            if (!wbin_is_ok(result2)) return result2;
            data = result2.value.next_data;
            data = wbin_decode_leb128(data, &wdata->len);
            wdata->bytes = data;
            data += wdata->len;
            break;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_datas(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len-- > 0) {
        WasmData wdata;
        wmod_data_init(&wdata);
        WasmDecodeResult result = wbin_decode_data(data, &wdata);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_data(wmod, &wdata);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_datacount(void *data, WasmModule *wmod) {
    return wbin_ok(wbin_decode_leb128(data, &wmod->meta.datacount));
}

WasmDecodeResult wbin_decode_funcidx_refs(void *data, VEC(WasmExpr) *exprs) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len-- > 0) {
        WasmExpr expr;
        vec_init(&expr);

        WasmInstruction instr;
        instr.opcode = WasmOpRefFunc;
        data = wbin_decode_leb128(data, &instr.params.ref_func.funcidx);
        wmod_expr_push_back_instruction(&expr, &instr);

        vec_push_back(exprs, sizeof(WasmExpr), &expr);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_elemkind(void *data, WasmRefType *reftype) {
    *reftype = WasmRefFunc;
    return wbin_decode_zero(data);
}

WasmDecodeResult wbin_decode_exprs(void *data, VEC(WasmExpr) *exprs) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len-- > 0) {
        WasmExpr expr;
        vec_init(&expr);
        WasmDecodeResult result = wbin_decode_expr(data, &expr);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        vec_push_back(exprs, sizeof(WasmExpr), &expr);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_elem(void *data, WasmElem *elem) {
    uint32_t tag;
    data = wbin_decode_leb128(data, &tag);

    switch (tag) {
        case 0: {
            elem->elemmode.kind = WasmElemModeActive;
            elem->reftype = WasmRefFunc;
            elem->elemmode.value.active.tableidx = 0;
            WasmDecodeResult result = wbin_decode_expr(data, &elem->elemmode.value.active.offset_expr);
            if (!wbin_is_ok(result)) return result;
            data = result.value.next_data;
            return wbin_decode_funcidx_refs(data, &elem->init);
        }
        case 1: {
            elem->elemmode.kind = WasmElemModePassive;
            WasmDecodeResult kind_result = wbin_decode_elemkind(data, &elem->reftype);
            if (!wbin_is_ok(kind_result)) return kind_result;
            data = kind_result.value.next_data;
            return wbin_decode_funcidx_refs(data, &elem->init);
        }
        case 2: {
            elem->elemmode.kind = WasmElemModeActive;
            data = wbin_decode_leb128(data, &elem->elemmode.value.active.tableidx);
            WasmDecodeResult result = wbin_decode_expr(data, &elem->elemmode.value.active.offset_expr);
            if (!wbin_is_ok(result)) return result;
            data = result.value.next_data;
            WasmDecodeResult kind_result = wbin_decode_elemkind(data, &elem->reftype);
            if (!wbin_is_ok(kind_result)) return kind_result;
            data = result.value.next_data;
            return wbin_decode_funcidx_refs(data, &elem->init);
        }
        case 3: {
            elem->elemmode.kind = WasmElemModeDeclarative;
            WasmDecodeResult kind_result = wbin_decode_elemkind(data, &elem->reftype);
            if (!wbin_is_ok(kind_result)) return kind_result;
            return wbin_decode_funcidx_refs(data, &elem->init);
        }
        case 4: {
            elem->elemmode.kind = WasmElemModeActive;
            elem->elemmode.value.active.tableidx = 0;
            elem->reftype = WasmRefFunc;
            WasmDecodeResult offset_result = wbin_decode_expr(data, &elem->elemmode.value.active.offset_expr);
            if (!wbin_is_ok(offset_result)) return offset_result;
            data = offset_result.value.next_data;
            return wbin_decode_exprs(data, &elem->init);
        }
        case 5: {
            elem->elemmode.kind = WasmElemModePassive;
            WasmDecodeResult ref_result = wbin_decode_reftype(data, &elem->reftype);
            if (!wbin_is_ok(ref_result)) return ref_result;
            data = ref_result.value.next_data;
            return wbin_decode_exprs(data, &elem->init);
        }
        case 6: {
            elem->elemmode.kind = WasmElemModeActive;
            data = wbin_decode_leb128(data, &elem->elemmode.value.active.tableidx);
            WasmDecodeResult offset_result = wbin_decode_expr(data, &elem->elemmode.value.active.offset_expr);
            if (!wbin_is_ok(offset_result)) return offset_result;
            data = offset_result.value.next_data;
            WasmDecodeResult ref_result = wbin_decode_reftype(data, &elem->reftype);
            if (!wbin_is_ok(ref_result)) return ref_result;
            data = ref_result.value.next_data;
            return wbin_decode_exprs(data, &elem->init);
        }
        case 7: {
            elem->elemmode.kind = WasmElemModeDeclarative;
            WasmDecodeResult ref_result = wbin_decode_reftype(data, &elem->reftype);
            if (!wbin_is_ok(ref_result)) return ref_result;
            data = ref_result.value.next_data;
            return wbin_decode_exprs(data, &elem->init);
        }
        default:
            return wbin_err(WasmDecodeErrInvalidElem, tag);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_elems(void *data, WasmModule *wmod) {
    uint32_t len;
    data = wbin_decode_leb128(data, &len);

    while (len-- > 0) {
        WasmElem elem;
        wmod_elem_init(&elem);
        WasmDecodeResult result = wbin_decode_elem(data, &elem);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        wmod_push_back_elem(wmod, &elem);
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_section(WasmSectionId id, void *section, WasmModule *wmod) {
    switch (id) {
        case SectionIdType:
            return wbin_decode_types(section, wmod);
        case SectionIdFunction:
            return wbin_decode_funcs(section, wmod);
        case SectionIdTable:
            return wbin_decode_tables(section, wmod);
        case SectionIdMemory:
            return wbin_decode_mems(section, wmod);
        case SectionIdImport:
            return wbin_decode_imports(section, wmod);
        case SectionIdExport:
            return wbin_decode_exports(section, wmod);
        case SectionIdStart:
            return wbin_decode_start(section, wmod);
        case SectionIdCode:
            return wbin_decode_codes(section, wmod);
        case SectionIdGlobal:
            return wbin_decode_globals(section, wmod);
        case SectionIdData:
            return wbin_decode_datas(section, wmod);
        case SectionIdDataCount:
            return wbin_decode_datacount(section, wmod);
        case SectionIdElement:
            return wbin_decode_elems(section, wmod);
        case SectionIdCustom:
            return wbin_ok(section);
        default:
            return wbin_err(WasmDecodeErrUnknownSectionId, id);
    }
}

WasmDecodeResult wbin_decode_sections(off_t size, WasmSectionHeader *section, WasmModule *wmod) {
    while (size > 0) {
        uint32_t len = 0;
        void* data = wbin_decode_leb128(section->data, &len);
        WasmDecodeResult sec_result = wbin_decode_section(section->section_id, data, wmod);
        if (!wbin_is_ok(sec_result)) return sec_result;
        size -= (len + 1);
        section = (WasmSectionHeader*) data + len;
    }
    return wbin_ok(section);
}

WasmDecodeResult wbin_read_module(char *path, WasmModule *wmod) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return wbin_err_io(errno);

    struct stat stats;
    int stat_err = fstat(fd, &stats);
    if (stat_err == -1) return wbin_err_io(errno);

    WasmHeader* data = mmap(NULL, stats.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if ((ptrdiff_t) data == -1) return wbin_err_io(errno);

    int close_err = close(fd);
    if (close_err != 0) return wbin_err_io(errno);

    return wbin_decode_module(stats.st_size, data, wmod);
}

WasmDecodeResult wbin_decode_module(size_t size, WasmHeader *header, WasmModule *wmod) {
    if (
        size < sizeof(WasmHeader)
        || header->magic_bytes[0] != '\0'
        || header->magic_bytes[1] != 'a'
        || header->magic_bytes[2] != 's'
        || header->magic_bytes[3] != 'm'
    ) return wbin_err(WasmDecodeErrMagicBytes, 0);

    wmod->meta.version = header->version;
    if (header->version != 1) return wbin_err(WasmDecodeErrUnsupportedVersion, 0);

    return wbin_decode_sections(size - sizeof(WasmHeader), header->sections, wmod);
}

char *wbin_explain_error_code(WasmDecodeResult result) {
    switch (result.value.error.code) {
        case WasmDecodeErrIo:
            return "unable to open file";
        case WasmDecodeErrMagicBytes:
            return "not a wasm module";
        case WasmDecodeErrUnsupportedVersion:
            return "unsupported version";
        case WasmDecodeErrOom:
            return "out of memory";
        case WasmDecodeErrUnknownSectionId:
            return "unknown section id";
        case WasmDecodeErrInvalidType:
            return "invalid type";
        case WasmDecodeErrUnknownValueType:
            return "unknown value type";
        case WasmDecodeErrInvalidImport:
            return "invalid import";
        case WasmDecodeErrInvalidGlobalMutability:
            return "invalid global mutability";
        case WasmDecodeErrInvalidExport:
            return "invalid export";
        case WasmDecodeErrInvalidTableInstr:
            return "unknown table instruction";
        case WasmDecodeErrExpectedZero:
            return "expected zero bytes";
        case WasmDecodeErrUnknownOpcode:
            return "unknown opcode";
        case WasmDecodeErrInvalidElem:
            return "invalid elem";
        default:
            return "unknown error code";
    }
}

char *wbin_explain_error_cause(WasmDecodeResult result) {
    switch (result.value.error.code) {
        case WasmDecodeErrIo:
            return strerror(result.value.error.cause);
        case WasmDecodeErrUnknownOpcode:
            return "...";
        default:
            return "";
    }
}

bool wbin_is_ok(WasmDecodeResult result) {
    return result.state == WasmDecodeOk;
}

bool wbin_is_err(WasmDecodeResult result, WasmDecodeErrorCode code) {
    return result.state == WasmDecodeErr
        && result.value.error.code == code;
}

bool wbin_error_has_cause(WasmDecodeResult result) {
    switch (result.value.error.code) {
        case WasmDecodeErrIo:
        case WasmDecodeErrUnknownOpcode:
            return true;
        default:
            return false;
    }
}
