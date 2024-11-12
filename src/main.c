
#include <stdio.h>
#include <stdlib.h>
#include <sys/types.h>
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

int main(int argc, char *argv[]) {

    CliArgs args;
    WasmModule wmod;
    WasmStore store;
    WasmModuleInst winst;

    wmod_init(&wmod);
    wrun_store_init(&store);

    cli_parse_or_exit(&args, argc, argv);
    wbin_read_module_or_exit(&args, &wmod);

    wrun_instantiate_module(&wmod, &store, &winst);

    wmod_dump(&wmod);

    return 0;
}
