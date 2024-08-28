
#include "wbin.h"
#include "wmod.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/_types/_u_int32_t.h>
#include <sys/_types/_u_int8_t.h>
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

void *wbin_take_byte(void *data, u_int8_t *out) {
    u_int8_t *bytes = data;
    *out = bytes[0];
    return bytes + 1;
}

void *wbin_decode_leb128(u_leb128_prefixed data, u_int32_t *out) {
    u_int32_t shift = 0;
    size_t byte_idx = 0;
    *out = 0;
    while (true) {
        u_int8_t byte = data[byte_idx];
        *out |= (byte & ~(1 << 7)) << shift;
        if ((byte & (1 << 7)) == 0) {
            break;
        };
        shift += 7;
        byte_idx++;
    }
    return data + byte_idx + 1;
}

void *wbin_decode_leb128_signed(u_leb128_prefixed data, u_int32_t *out) {
    int64_t result = 0;
    u_int32_t shift = 0;

    size_t idx = 0;
    u_int8_t byte = data[idx];
    do {
      byte = data[idx];
      result |= (byte & ~(1 << 7)) << shift;
      shift += 7;
      idx++;
    } while ((byte & (1 << 7)) != 0);

    if ((shift < 64) && ((byte & 0x40) == 1)) {
        result |= (~0 << shift);
    }

    *out = result;

    return data + idx + 1;
}

WasmDecodeResult wbin_decode_reftype(void *data, WasmRefType *out) {
    u_int8_t tag;
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
    u_int8_t tag;
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
    u_int32_t len = 0;
    data = wbin_decode_leb128(data, &len);
    while (len > 0) {
        WasmValueType valtype;
        WasmDecodeResult valtype_result = wbin_decode_val_type(data, &valtype);
        if (!wbin_is_ok(valtype_result)) return valtype_result;
        size_t idx = wmod_result_type_push_back(out, &valtype);
        data = valtype_result.value.next_data;
        len--;
    }
    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_type(void *data, WasmFuncType *out) {
    u_int8_t tag;
    data = wbin_take_byte(data, &tag);
    if (tag != 0x60) return wbin_err(WasmDecodeErrInvalidType, 0);
    WasmDecodeResult input_result = wbin_decode_result_type(data, &out->input_type);
    if (!wbin_is_ok(input_result)) return input_result;
    return wbin_decode_result_type(input_result.value.next_data, &out->output_type);
}

WasmDecodeResult wbin_decode_types(void *data, WasmModule *wmod) {
    u_int32_t len = 0;
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
    u_int32_t len = 0;
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
    u_int8_t tag;
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
    u_int32_t len;
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
    u_int32_t len;
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
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);
    name->len = len;
    name->bytes = data;
    return wbin_ok(data + len);
}

WasmDecodeResult wbin_decode_global_mutability(void *data, WasmGlobalMutability *mut) {
    u_int8_t tag;
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

WasmDecodeResult wbin_decode_global(void *data, WasmGlobalType *global) {
    WasmDecodeResult val_result = wbin_decode_val_type(data, &global->valtype);
    if (!wbin_is_ok(val_result)) return val_result;
    data = val_result.value.next_data;
    return wbin_decode_global_mutability(data, &global->mut);
}

WasmDecodeResult wbin_decode_import_desc(void *data, WasmImportDesc *desc) {
    u_int8_t tag;
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
            return wbin_decode_global(data, &desc->value.global);
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
    u_int32_t len;
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
    u_int8_t tag;
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
    u_int32_t len;
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
    u_int32_t len;
    data =  wbin_decode_leb128(data, &len);
    for (size_t i = 0; i < len; i++) {
        u_int32_t n;
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
    if (*(u_int8_t*)data == 0x40) {
        blocktype->kind = WasmBlockTypeEmpty;
        return wbin_ok((u_int8_t*)data + 1);
    }
    WasmDecodeResult val_result = wbin_decode_val_type(data, &blocktype->value.valtype);
    if (wbin_is_ok(val_result)) {
        blocktype->kind = WasmBlockTypeVal;
        return val_result;
    }
    if (!wbin_is_err(val_result, WasmDecodeErrInvalidType)) return val_result;

    blocktype->kind = WasmBlockTypeIdx;
    data = wbin_decode_leb128_signed(data, &blocktype->value.typeidx);

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
    u_int32_t len;
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

WasmDecodeResult wbin_decode_instr(void *data, WasmInstruction *ins) {
    u_int8_t tag;
    data = wbin_take_byte(data, &tag);

    switch (tag) {
        case 0x00:
            ins->opcode = WasmOpUnreachable;
            break;
        case 0x01:
            ins->opcode = WasmOpNop;
            break;
        case 0x02:
            ins->opcode = WasmOpBlock;
            vec_init(&ins->params.block.expr);
            return wbin_decode_block(data, &ins->params.block);
        case 0x03:
            ins->opcode = WasmOpLoop;
            vec_init(&ins->params.block.expr);
            return wbin_decode_block(data, &ins->params.block);
        case 0x04:
            ins->opcode = WasmOpIf;
            vec_init(&ins->params._if.then_body);
            vec_init(&ins->params._if.else_body);
            return wbin_decode_if(data, &ins->params._if);
        case 0x05:
            ins->opcode = WasmOpElse;
            break;
        case 0x0C:
            ins->opcode = WasmOpBreak;
            return wbin_decode_break(data, &ins->params._break);
        case 0x0D:
            ins->opcode = WasmOpBreakIf;
            return wbin_decode_break(data, &ins->params._break);
        case 0x0E:
            ins->opcode = WasmOpBreakTable;
            vec_init(&ins->params.break_table.labels);
            return wbin_decode_break_table(data, &ins->params.break_table);
        case 0x0F:
            ins->opcode = WasmOpReturn;
            break;
        case 0x10:
            ins->opcode = WasmOpCall;
            return wbin_decode_call(data, &ins->params.call);
        case 0x11:
            ins->opcode = WasmOpCallIndirect;
            return wbin_decode_call_indirect(data, &ins->params.call_indirect);
        // END
        default:
        case 0x0B:
            ins->opcode = WasmOpExprEnd;
            break;
    }

    return wbin_ok(data);
}

WasmDecodeResult wbin_decode_expr(void *data, WasmExpr *expr) {
    WasmInstruction instr;
    while (true) {
        WasmDecodeResult result = wbin_decode_instr(data, &instr);
        if (!wbin_is_ok(result)) return result;
        data = result.value.next_data;
        if (instr.opcode == WasmOpExprEnd) {
            break;
        } else {
            wmod_expr_push_back_instruction(expr, &instr);
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
    u_int32_t len;
    data = wbin_decode_leb128(data, &len);

    WasmFunc *func = wmod->funcs.ptr;
    for (size_t i = 0; i < len; i++) {
        u_int32_t code_len;
        data = wbin_decode_leb128(data, &code_len);
        WasmDecodeResult result = wbin_decode_code(data, &func[i]);
        if (!wbin_is_ok(result)) return result;
        data += code_len;
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
        case SectionIdCustom:
        case SectionIdGlobal:
        case SectionIdElement:
        case SectionIdData:
        case SectionIdDataCount:
            return wbin_ok(section);
        default:
            return wbin_err(WasmDecodeErrUnknownSectionId, id);
    }
}

WasmDecodeResult wbin_decode_sections(off_t size, WasmSectionHeader *section, WasmModule *wmod) {
    while (size > 0) {
        u_int32_t len = 0;
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
    if ((size_t) data == -1) return wbin_err_io(errno);

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
        default:
            return "unknown error code";
    }
}

char *wbin_explain_error_cause(WasmDecodeResult result) {
    switch (result.value.error.code) {
        case WasmDecodeErrIo:
            return strerror(result.value.error.cause);
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
    return result.state == WasmDecodeErrIo;
}
