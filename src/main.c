
#include <stdio.h>
#include "cli.h"
#include "wmod.h"

int main(int argc, char *argv[]) {
    CliArgs args;
    int cli_err = cli_parse(&args, argc, argv);

    if (cli_err != 0) {
        printf("Failed to parse arguments: %s\n", cli_str_error(cli_err));
        printf("Usage:\n\t%s\n", cli_usage_str());
        return 1;
    }

    printf("Loading wasm module at %s\n", args.path);
    WasmModule wmod;
    int wmod_err = wmod_read(&wmod, args.path);

    if (wmod_err != 0) {
        printf("Failed to load wasm module at %s: %s\n", args.path, wmod_str_error(wmod_err));
        return 2;
    }

    return 0;
}
