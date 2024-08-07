
#include "cli.h"

int cli_parse(CliArgs *args, int argc, char *argv[]) {
    if (argc < 2) {
        return 1;
    }

    args->path = argv[1];

    return 0;
}
