
#include <stdio.h>
#include <string.h>
#include "cli.h"

const int CLI_ERR_NO_PATH = 1;
const int CLI_ERR_NO_INVOKE = 2;
const int CLI_ERR_INCOMPLETE_OPTION = 3;
const int CLI_ERR_UNKNOWN_FLAG = 4;
const int CLI_ERR_TOO_MANY_ARGS = 5;

void cli_init(CliArgs *args) {
    args->help = false;
    args->path = NULL;
    args->invoke = NULL;
}

int cli_parse(CliArgs *args, int argc, char *argv[]) {
    cli_init(args);

    for (int i = 1; i < argc; i++) {
        char* opt = argv[i];

        if (strcmp(opt, "-h") == 0 || strcmp(opt, "--help") == 0) {
            args->help = true;
            return 0;
        } else if (strcmp(opt, "-I") == 0 || strcmp(opt, "--invoke") == 0) {
            if (i + 1 >= argc) {
                return CLI_ERR_INCOMPLETE_OPTION;
            }
            args->invoke = argv[i+1];
            i++;
        } else if (strncmp(opt, "-", 1) == 0) {
            return CLI_ERR_UNKNOWN_FLAG;
        } else {
            if (args->path != NULL) {
                return CLI_ERR_TOO_MANY_ARGS;
            }
            args->path = argv[i];
        }
    }

    if (args->path == NULL) {
        return CLI_ERR_NO_PATH;
    }

    if (args->invoke == NULL) {
        return CLI_ERR_NO_INVOKE;
    }

    return 0;
}

char *cli_str_error(int err) {
    switch (err) {
        case CLI_ERR_NO_PATH:
            return "missing path";
        case CLI_ERR_NO_INVOKE:
            return "--invoke is required";
        case CLI_ERR_INCOMPLETE_OPTION:
            return "--invoke missing <NAME>";
        case CLI_ERR_UNKNOWN_FLAG:
            return "unknown flag";
        case CLI_ERR_TOO_MANY_ARGS:
            return "too many args";
        default:
            return "unknown error";
    }
}

char *cli_usage_str() {
    return
        "semblance <MODULE.wasm>\n"
        "\n"
        "Options:\n"
        "\t-h, --help\t\tPrint this help text\n"
        "\t-I, --invoke <NAME>\tInvoke the function exported as $NAME\n";
}
