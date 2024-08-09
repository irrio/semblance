
#include <stdio.h>
#include <sys/types.h>
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

    WmodErr wmod_err;
    WasmModule *wmod = wmod_read(args.path, &wmod_err);

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

    printf("%s has %lld bytes\n", args.path, wmod->total_size);

    char *magic = wmod->raw_data->magic_bytes;
    printf("magic bytes: \"%c%c%c%c\"\n", magic[0], magic[1], magic[2], magic[3]);

    u_int32_t version = wmod->raw_data->version;
    printf("version: %d\n", version);

    size_t num_sections = wmod->num_sections;
    printf("sections: %zu\n", num_sections);

    return 0;
}
