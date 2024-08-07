
#include <stdio.h>
#include "cli.h"

const int CLI_ERR_NO_PATH = 1;

int cli_parse(CliArgs *args, int argc, char *argv[]) {
    if (argc < 2) {
        return CLI_ERR_NO_PATH;
    }

    args->path = argv[1];

    return 0;
}

char *cli_str_error(int err) {
    switch (err) {
        case CLI_ERR_NO_PATH:
            return "missing path";
        default:
            return "unknown error";
    }
}

char *cli_usage_str() {
    return "semblance [path]";
}
