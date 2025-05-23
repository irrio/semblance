
#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include "cli.h"
#include "wmod.h"
#include "wbin.h"
#include "wrun.h"

void cli_parse_or_exit(CliArgs *args, int argc, char *argv[]) {
    int cli_err = cli_parse(args, argc, argv);
    if (cli_err != 0) {
        printf("Failed to parse arguments: %s\n", cli_str_error(cli_err));
        printf("Usage:\n\t%s\n", cli_usage_str());
        exit(1);
    }
}

void wbin_read_module_or_exit(CliArgs *args, WasmModule *wmod) {
    WasmDecodeResult result = wbin_read_module(args->path, wmod);
    if (!wbin_is_ok(result)) {
        printf(
            "Failed to load wasm module at \"%s\": %s",
            args->path,
            wbin_explain_error_code(result)
        );
        if (wbin_error_has_cause(result)) {
            printf(" (%s)", wbin_explain_error_cause(result));
        }
        printf("\n");
        exit(2);
    }
}

int cli_invoke_argv_parse_num(WasmNumType type, char *arg, WasmNumValue *val) {
    switch (type) {
        case WasmNumI32: {
            char *end;
            long num = strtol(arg, &end, 10);
            //if (arg != end) return 1;
            if (num > INT_MAX) return 2;
            val->i32 = num;
            break;
        }
        case WasmNumI64: {
            char *end;
            long num = strtol(arg, &end, 10);
            //if (arg != end) return 1;
            val->i64 = num;
            break;
        }
        case WasmNumF32:
            return 1;
        case WasmNumF64:
            return 1;
    }
    return 0;
}

int cli_invoke_argv_parse(WasmResultType *type, char **argv, VEC(WasmValue) *out) {
    for (size_t i = 0; i < type->len; i++) {
        WasmValueType *vtype = vec_at(type, sizeof(WasmValueType), i);
        switch (vtype->kind) {
            case WasmValueTypeNum: {
                WasmValue val;
                int err = cli_invoke_argv_parse_num(vtype->value.num, argv[i], &val.num);
                if (err) return err;
                vec_push_back(out, sizeof(WasmValue), &val);
                break;
            }
            default:
                return 1;
        }
    }
    return 0;
}

VEC(WasmValue) hostcall_puts(WasmStore *store, VEC(WasmValue) *args) {
    VEC(WasmValue) out;
    vec_init(&out);

    WasmValue *arg = args->ptr;
    int32_t offset = arg[0].num.i32;

    WasmMemInst *mem = store->mems.ptr;
    void *data = mem[0].data.ptr;

    printf("%s\n", (char*)(data + offset));

    return out;
}

WasmExternVal register_hostcall_puts(WasmStore *store) {
    WasmFuncType puts_type;
    vec_init(&puts_type.input_type);
    WasmValueType arg1 = {
        .kind = WasmValueTypeNum,
        .value.num = WasmNumI32
    };
    vec_push_back(&puts_type.input_type, sizeof(WasmValueType), &arg1);
    vec_init(&puts_type.output_type);
    wasm_func_addr_t putsaddr = wrun_store_alloc_hostfunc(store, puts_type, hostcall_puts);
    WasmExternVal out = {
        .kind = WasmExternValFunc,
        .val.func = putsaddr
    };
    return out;
}

int main(int argc, char *argv[]) {

    CliArgs args;
    WasmModule wmod;
    WasmStore store;

    wmod_init(&wmod);
    wrun_store_init(&store);

    cli_parse_or_exit(&args, argc, argv);

    if (args.help) {
        printf("%s\n", cli_usage_str());
        return 0;
    }

    wbin_read_module_or_exit(&args, &wmod);

    VEC(WasmExternVal) imports;
    vec_init(&imports);

    for (size_t i = 0; i < wmod.imports.len; i++) {
        WasmImport *import = vec_at(&wmod.imports, sizeof(WasmImport), i);
        if (wmod_name_eq(&import->module_name, "env") && wmod_name_eq(&import->item_name, "puts")) {
            WasmExternVal func_puts = register_hostcall_puts(&store);
            vec_push_back(&imports, sizeof(WasmExternVal), &func_puts);
        }
    }

    WasmModuleInst *winst = wrun_instantiate_module(&wmod, &store, &imports);

    WasmExternVal export = wrun_resolve_export(winst, args.invoke);
    assert(export.kind == WasmExternValFunc);
    WasmFuncInst *finst = vec_at(&store.funcs, sizeof(WasmFuncInst), export.val.func - 1);

    VEC(WasmValue) fn_args;
    vec_init(&fn_args);
    int parse_err = cli_invoke_argv_parse(&finst->functype.input_type, args.invoke_argv, &fn_args);
    if (parse_err) {
        printf("failed to parse invoke args");
        exit(3);
    }

    DynamicWasmResult wres = wrun_invoke_func(export.val.func, &fn_args, &store);
    wrun_result_dump_dynamic(&wres);

    return wres.result.kind != Ok;
}
