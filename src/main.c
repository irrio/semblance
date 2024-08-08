
#include <stdio.h>
#include <sys/_types/_u_int32_t.h>
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

    WasmModule wmod;
    WmodErr wmod_err = wmod_read(&wmod, args.path);

    if (wmod_failed(wmod_err)) {
        printf(
            "Failed to load wasm module at %s: %s",
            args.path,
            wmod_str_error(wmod_err)
        );
        if (wmod_err.cause) {
            printf(" (%s)", wmod_str_error_cause(wmod_err));
        }
        printf("\n");
        return 2;
    }

    printf("%s has %lld bytes\n", args.path, wmod.size);

    char *magic = wmod.data->magic_bytes;
    printf("magic bytes: \"%c%c%c%c\"\n", magic[0], magic[1], magic[2], magic[3]);

    u_int32_t version = wmod.data->version;
    printf("version: %d\n", version);

    return 0;
}
