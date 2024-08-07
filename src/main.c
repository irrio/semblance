
#include <stdio.h>
#include "cli.h"

int main(int argc, char *argv[]) {
    CliArgs args;
    int cli_err = cli_parse(&args, argc, argv);

    if (cli_err != 0) {
        printf("Failed to parse arguments: %d\n", cli_err);
        return 1;
    }

    printf("Loading wasm module at %s\n", args.path);

    return 0;
}
