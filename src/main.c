
#include <stdio.h>
#include <sys/types.h>
#include "cli.h"
#include "wmod.h"
#include "wbin.h"

int main(int argc, char *argv[]) {
    CliArgs args;
    int cli_err = cli_parse(&args, argc, argv);

    if (cli_err != 0) {
        printf("Failed to parse arguments: %s\n", cli_str_error(cli_err));
        printf("Usage:\n\t%s\n", cli_usage_str());
        return 1;
    }

    WasmModule wmod;
    WasmDecodeResult result = wbin_read_module(args.path, &wmod);

    if (!wbin_is_ok(result)) {
        printf(
            "Failed to load wasm module at \"%s\": %s",
            args.path,
            wbin_explain_error_code(result)
        );
        if (wbin_error_has_cause(result)) {
            printf(" (%s)", wbin_explain_error_cause(result));
        }
        printf("\n");
        return 2;
    }

    return 0;
}
