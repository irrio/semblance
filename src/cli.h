
#pragma once

#include <stdbool.h>

typedef struct {
    char *path;
    char *invoke;
    int invoke_argc;
    char **invoke_argv;
    bool help;
} CliArgs;

int cli_parse(CliArgs *args, int argc, char *argv[]);

void cli_debug(CliArgs *args);

char *cli_str_error(int err);

char *cli_usage_str();
